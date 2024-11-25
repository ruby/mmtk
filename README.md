# MMTk Bindings for Ruby

This repository holds the [MMTk](https://www.mmtk.io/) bindings for Ruby. The binding plugs into Ruby using the garbage collector API implemented in [[Feature #20470]](https://bugs.ruby-lang.org/issues/20470). This API allows Ruby to use alternative garbage collector implementations such as MMTk.

## Getting started

### Building

Since this repository is sync'd into [ruby/ruby](https://github.com/ruby/ruby/), you can choose to either build it in this repository or build it in ruby/ruby. If you're building it in this repository, note that it is only expected to work on the latest commit of Ruby master.

To build `librubygc.mmtk.so` locally, run the following:

```sh
  rake compile
```
This command will:

   - **Clone the Ruby repository**: If you don't already have it, the script clones Ruby into `res/ruby`.
   - **Copy MMTk GC code**: It copies the MMTk garbage collector code into the Ruby source tree.
   - **Configure and build Ruby**: Sets up the build configurations to integrate MMTk with Ruby's garbage collector API.
   - **Compiling the MMTk shared library**: Builds `librubygc.mmtk.so` using Rust's Cargo build system.
   - **Install the shared library**: Copies `librubygc.mmtk.so` into the `mod_gc` directory.

Once it's done, you'll have a shared object `mod_gc/librubygc.mmtk.so` (`modgc/librubygc.mmtk.bundle` on macOS), that can be loaded into any Ruby interpeter built `--with-shared-gc` support. 

Successful builds, when loaded, will display `+GC[mmtk]`  in the Ruby version string. This indicates that Ruby has been compiled with shared GC support and that MMTk is loaded.

```sh
  RUBY_GC_LIBRARY=mmtk ruby -v
  ruby 3.4.0dev (2024-11-25T13:05:23Z master f127bcb829) +PRISM +GC[mmtk] [arm64-darwin24]
```

**Note**: Make sure you have all the necessary dependencies installed:

- **Rust toolchain**: Required for building MMTk.
- **Ruby build dependencies**: Libraries and tools needed to compile Ruby from source.
- **Rake**: Ruby's build program, used to run the compile task.

That's it! You're all set to experiment with different GC algorithms in Ruby using MMTk.









### Running

After building Ruby and the MMTk bindings, run Ruby with `RUBY_GC_LIBRARY=mmtk` environment variable. You can also configure the following environment variables:

- `MMTK_PLAN=<NoGC|MarkSweep>`: Configures the GC algorithm used by MMTk. Defaults to `MarkSweep`.
- `MMTK_HEAP_MODE=<fixed|dynamic>`: Configures the MMTk heap used. `fixed` is a fixed size heap, `dynamic` is a dynamic sized heap that will grow and shrink in size based on heuristics using the [MemBalancer](https://dl.acm.org/doi/pdf/10.1145/3563323) algorithm. Defaults to `dynamic`.
- `MMTK_HEAP_MIN=<size>`: Configures the lower bound in heap memory usage by MMTk. Only valid when `MMTK_HEAP_MODE=dynamic`. `size` is in bytes, but you can also append `KiB`, `MiB`, `GiB` for larger sizes. Defaults to 1MiB.
- `MMTK_HEAP_MAX=<size>`: Configures the upper bound in heap memory usage by MMTk. Once this limit is reached and no objects can be garbage collected, it will crash with an out-of-memory. `size` is in bytes, but you can also append `KiB`, `MiB`, `GiB` for larger sizes. Defaults to 80% of your system RAM.

## Code structure

TODO
