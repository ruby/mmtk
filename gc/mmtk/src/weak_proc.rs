use std::sync::Mutex;

use mmtk::scheduler::GCWork;
use mmtk::scheduler::GCWorker;
use mmtk::scheduler::WorkBucketStage;
use mmtk::util::ObjectReference;
use mmtk::vm::ObjectTracerContext;

use crate::Ruby;
use crate::abi::GCThreadTLS;
use crate::upcalls;

pub struct WeakProcessor {
    non_parallel_obj_free_candidates: Mutex<ObjFreeCandidates>,
    parallel_obj_free_candidates: Vec<Mutex<ObjFreeCandidates>>,

    /// Objects that needs `obj_free` called when dying.
    /// If it is a bottleneck, replace it with a lock-free data structure,
    /// or add candidates in batch.
    weak_references: Mutex<Vec<ObjectReference>>,
}

/// Candidates for `obj_free`, with a generation watermark for generational
/// (StickyImmix) collection.
///
/// `objects[..mature_len]` are mature: they survived at least one GC.  Under
/// sticky mark bits such objects are considered reachable in nursery GCs
/// without being traced, and nursery GCs never move them, so nursery GCs
/// only need to scan the young suffix `objects[mature_len..]`.  Full GCs
/// scan the whole list.
struct ObjFreeCandidates {
    objects: Vec<ObjectReference>,
    mature_len: usize,
}

impl ObjFreeCandidates {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
            mature_len: 0,
        }
    }
}

impl Default for WeakProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl WeakProcessor {
    pub fn new() -> Self {
        Self {
            non_parallel_obj_free_candidates: Mutex::new(ObjFreeCandidates::new()),
            parallel_obj_free_candidates: vec![Mutex::new(ObjFreeCandidates::new())],
            weak_references: Mutex::new(Vec::new()),
        }
    }

    pub fn init_parallel_obj_free_candidates(&mut self, num_workers: usize) {
        debug_assert_eq!(self.parallel_obj_free_candidates.len(), 1);

        for _ in 1..num_workers {
            self.parallel_obj_free_candidates
                .push(Mutex::new(ObjFreeCandidates::new()));
        }
    }

    /// Add a batch of objects as candidates for `obj_free`.
    ///
    /// Amortizes mutex acquisition over the entire batch. Called when a
    /// mutator's local buffer is flushed (buffer full or stop-the-world).
    pub fn add_obj_free_candidates_batch(
        &self,
        objects: &[ObjectReference],
        can_parallel_free: bool,
    ) {
        if objects.is_empty() {
            return;
        }

        if can_parallel_free {
            let num_buckets = self.parallel_obj_free_candidates.len();
            for idx in 0..num_buckets {
                let mut bucket = self.parallel_obj_free_candidates[idx].lock().unwrap();
                for (i, &obj) in objects.iter().enumerate() {
                    if i % num_buckets == idx {
                        bucket.objects.push(obj);
                    }
                }
            }
        } else {
            self.non_parallel_obj_free_candidates
                .lock()
                .unwrap()
                .objects
                .extend_from_slice(objects);
        }
    }

    pub fn get_all_obj_free_candidates(&self) -> Vec<ObjectReference> {
        let mut all_obj_free_candidates = self
            .non_parallel_obj_free_candidates
            .lock()
            .unwrap()
            .objects
            .to_vec();

        for candidates_mutex in &self.parallel_obj_free_candidates {
            all_obj_free_candidates
                .extend(candidates_mutex.lock().unwrap().objects.iter().copied());
        }

        std::mem::take(all_obj_free_candidates.as_mut())
    }

    pub fn add_weak_reference(&self, object: ObjectReference) {
        let mut weak_references = self.weak_references.lock().unwrap();
        weak_references.push(object);
    }

    pub fn weak_references_count(&self) -> usize {
        self.weak_references.lock().unwrap().len()
    }

    pub fn process_weak_stuff(
        &self,
        worker: &mut GCWorker<Ruby>,
        _tracer_context: impl ObjectTracerContext<Ruby>,
    ) {
        // The GC kind is fixed for the whole GC cycle, so query it once here.
        let nursery_gc = is_nursery_gc();

        worker.add_work(
            WorkBucketStage::VMRefClosure,
            ProcessNonParallelObjFreeCanadidates { nursery_gc },
        );

        for index in 0..self.parallel_obj_free_candidates.len() {
            worker.add_work(
                WorkBucketStage::VMRefClosure,
                ProcessParallelObjFreeCandidates { index, nursery_gc },
            );
        }

        worker.add_work(WorkBucketStage::VMRefClosure, ProcessWeakReferences);

        worker.add_work(WorkBucketStage::Prepare, UpdateFinalizerTable);

        let global_tables_count = crate::upcalls::global_tables_count();
        let work_packets = (0..global_tables_count)
            .map(|i| Box::new(UpdateGlobalTables { idx: i }) as _)
            .collect();

        worker.scheduler().work_buckets[WorkBucketStage::VMRefClosure].bulk_add(work_packets);

        worker.scheduler().work_buckets[WorkBucketStage::VMRefClosure]
            .bulk_add(vec![Box::new(UpdateWbUnprotectedObjectsList) as _]);
    }
}

/// Whether the current GC is a nursery (young-only) collection.  Always
/// false for non-generational plans.
fn is_nursery_gc() -> bool {
    (crate::mmtk().get_plan().generational())
        .is_some_and(|gen_plan| gen_plan.is_current_gc_nursery())
}

/// Process obj_free candidates, calling `obj_free` on the dead ones and
/// keeping the rest.
///
/// In a nursery GC, only the young suffix is scanned: entries in the mature
/// prefix survived a previous GC and are certainly still alive (sticky mark
/// bits), so they can neither die nor move. Everything is scanned in a full
/// GC.
fn process_obj_free_candidates(candidates: &mut ObjFreeCandidates, nursery_gc: bool) {
    let total = candidates.objects.len();
    let young_start = if nursery_gc { candidates.mature_len } else { 0 };

    // In-place compaction of the scanned range [young_start, total).
    let mut write = young_start;
    for read in young_start..total {
        let object = candidates.objects[read];
        if object.is_reachable() {
            // Forward and keep the candidate.
            let new_object = object.forward();
            trace!("Forwarding obj_free candidate: {object} -> {new_object}");
            candidates.objects[write] = new_object;
            write += 1;
        } else {
            upcalls::call_obj_free(object);
        }
    }
    candidates.objects.truncate(write);
    candidates.mature_len = write;

    debug!(
        "Processed obj_free candidates: {total} total, {} scanned, {} skipped mature, {} freed (nursery_gc={nursery_gc})",
        total - young_start,
        young_start,
        total - write,
    );
}

struct ProcessParallelObjFreeCandidates {
    index: usize,
    nursery_gc: bool,
}

impl GCWork<Ruby> for ProcessParallelObjFreeCandidates {
    fn do_work(&mut self, _worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        let mut obj_free_candidates = crate::binding().weak_proc.parallel_obj_free_candidates
            [self.index]
            .try_lock()
            .expect("Lock for parallel_obj_free_candidates should not be held");

        process_obj_free_candidates(&mut obj_free_candidates, self.nursery_gc);
    }
}

struct ProcessNonParallelObjFreeCanadidates {
    nursery_gc: bool,
}

impl GCWork<Ruby> for ProcessNonParallelObjFreeCanadidates {
    fn do_work(&mut self, _worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        let mut obj_free_candidates = crate::binding()
            .weak_proc
            .non_parallel_obj_free_candidates
            .try_lock()
            .expect("Lock for non_parallel_obj_free_candidates should not be held");

        process_obj_free_candidates(&mut obj_free_candidates, self.nursery_gc);
    }
}

struct ProcessWeakReferences;

impl GCWork<Ruby> for ProcessWeakReferences {
    fn do_work(&mut self, worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        if crate::mmtk().get_plan().current_gc_may_move_object() {
            let gc_tls: &mut GCThreadTLS = unsafe { GCThreadTLS::from_vwt_check(worker.tls) };

            let visit_object = |_worker, target_object: ObjectReference, _pin| {
                debug_assert!(
                    mmtk::memory_manager::is_mmtk_object(target_object.to_raw_address()).is_some(),
                    "Destination is not an MMTk object"
                );

                target_object
                    .get_forwarded_object()
                    .unwrap_or(target_object)
            };

            gc_tls
                .object_closure
                .set_temporarily_and_run_code(visit_object, || {
                    self.process_weak_references(true);
                })
        } else {
            self.process_weak_references(false);
        }
    }
}

impl ProcessWeakReferences {
    fn process_weak_references(&mut self, moving_gc: bool) {
        let mut weak_references = crate::binding()
            .weak_proc
            .weak_references
            .try_lock()
            .expect("Mutators should not be holding the lock.");

        weak_references.retain_mut(|object_ptr| {
            let object = object_ptr.get_forwarded_object().unwrap_or(*object_ptr);

            if object != *object_ptr {
                *object_ptr = object;
            }

            if object.is_reachable() {
                upcalls::handle_weak_references(object, moving_gc);

                true
            } else {
                false
            }
        });
    }
}

trait GlobalTableProcessingWork {
    fn process_table(&mut self);

    fn do_work(&mut self, worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(worker.tls) };

        // `hash_foreach_replace` depends on `gb_object_moved_p` which has to have the semantics
        // of `trace_object` due to the way it is used in `UPDATE_IF_MOVED`.
        let forward_object = |_worker, object: ObjectReference, _pin| {
            debug_assert!(
                mmtk::memory_manager::is_mmtk_object(object.to_raw_address()).is_some(),
                "{object} is not an MMTk object"
            );
            let result = object.forward();
            trace!("Forwarding reference: {object} -> {result}");
            result
        };

        gc_tls
            .object_closure
            .set_temporarily_and_run_code(forward_object, || {
                self.process_table();
            });
    }
}

struct UpdateFinalizerTable;
impl GlobalTableProcessingWork for UpdateFinalizerTable {
    fn process_table(&mut self) {
        crate::upcalls::update_finalizer_table();
    }
}
impl GCWork<Ruby> for UpdateFinalizerTable {
    fn do_work(&mut self, worker: &mut GCWorker<Ruby>, mmtk: &'static mmtk::MMTK<Ruby>) {
        GlobalTableProcessingWork::do_work(self, worker, mmtk);
    }
}

struct UpdateGlobalTables {
    idx: i32,
}
impl GlobalTableProcessingWork for UpdateGlobalTables {
    fn process_table(&mut self) {
        crate::upcalls::update_global_tables(
            self.idx,
            crate::mmtk().get_plan().current_gc_may_move_object(),
        )
    }
}
impl GCWork<Ruby> for UpdateGlobalTables {
    fn do_work(&mut self, worker: &mut GCWorker<Ruby>, mmtk: &'static mmtk::MMTK<Ruby>) {
        GlobalTableProcessingWork::do_work(self, worker, mmtk);
    }
}

struct UpdateWbUnprotectedObjectsList;

impl GCWork<Ruby> for UpdateWbUnprotectedObjectsList {
    fn do_work(&mut self, _worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        let mut objects = crate::binding().wb_unprotected_objects.try_lock().expect(
            "Someone is holding the lock of wb_unprotected_objects during weak processing phase?",
        );

        let old_objects = std::mem::take(&mut *objects);

        debug!("Updating {} WB-unprotected objects", old_objects.len());

        for object in old_objects {
            if object.is_reachable() {
                // Forward and add back to the candidate list.
                let new_object = object.forward();
                trace!("Forwarding WB-unprotected object: {object} -> {new_object}");
                objects.insert(new_object);
            } else {
                trace!("Removing WB-unprotected object from list: {object}");
            }
        }

        debug!("Retained {} live WB-unprotected objects.", objects.len());
    }
}

// Provide a shorthand `object.forward()`.
trait Forwardable {
    fn forward(&self) -> Self;
}

impl Forwardable for ObjectReference {
    fn forward(&self) -> Self {
        self.get_forwarded_object().unwrap_or(*self)
    }
}
