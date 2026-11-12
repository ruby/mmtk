#!/usr/bin/env python3

def enrich_meta_extra(log_processor, name, tid, ts, gc, wp, args):
    match name:
        case "process_obj_free_candidates":
            num_candidates, num_retained = [int(arg) for arg in args]
            num_freed = num_candidates - num_retained

            aggregate_dict = {
                "num_candidates": num_candidates,
                "num_retained": num_retained,
                "num_freed": num_freed,
            }

            if wp is not None:
                log_processor.add_aggregatable_arg(wp, name, aggregate_dict)
            if gc is not None:
                log_processor.add_aggregatable_arg(gc, name, aggregate_dict)

        case "process_weak_references":
            num_weakrefs, num_retained = [int(arg) for arg in args]
            num_removed = num_weakrefs - num_retained

            aggregate_dict = {
                "num_weakrefs": num_weakrefs,
                "num_retained": num_retained,
                "num_removed": num_removed,
            }

            if wp is not None:
                log_processor.add_aggregatable_arg(wp, name, aggregate_dict)
            if gc is not None:
                log_processor.add_aggregatable_arg(gc, name, aggregate_dict)

        case "pin_pinning_children":
            num_pinning_objs, num_pinned_objs, num_newly_pinned_objs = [int(arg) for arg in args]

            aggregate_dict = {
                "num_pinning_objs": num_pinning_objs,
                "num_pinned_objs": num_pinned_objs,
                "num_newly_pinned_objs": num_newly_pinned_objs,
            }

            if wp is not None:
                log_processor.add_aggregatable_arg(wp, name, aggregate_dict)
            if gc is not None:
                log_processor.add_aggregatable_arg(gc, name, aggregate_dict)
