# frozen_string_literal: true

require "fileutils"
require "net/http"
require "rbconfig"
require "rake/extensiontask"
require "rake/testtask"

task default: :"install:debug"

Rake::TestTask.new(:test) do |t|
  t.test_files = FileList["test/**/test_*.rb"]
end

Rake::ExtensionTask.prepend(Module.new do
  def binary(platform = nil)
    "librubygc.mmtk.#{RbConfig::CONFIG["DLEXT"]}"
  end
end)

extension_configuration = proc do |ext|
  ext.ext_dir = "gc/mmtk"
  ext.lib_dir = "tmp/binaries"
end

Rake::ExtensionTask.new(:debug, &extension_configuration)

Rake::ExtensionTask.new(:release) do |ext|
  extension_configuration.call(ext)
  ext.instance_variable_set(:@make, ext.send(:make) + " MMTK_BUILD=release")
end

namespace :install do
  install_task = proc do
    puts "mv #{Dir.glob("tmp/binaries/*").join} #{RbConfig::CONFIG["modular_gc_dir"]}"
    FileUtils.mv(Dir.glob("tmp/binaries/*"), RbConfig::CONFIG["modular_gc_dir"])
  end

  task(debug: :"compile:debug", &install_task)
  task(release: :"compile:release", &install_task)
end

task :cbindgen do
  Dir.chdir("gc/mmtk") do
    sh("cbindgen --config cbindgen.toml --output mmtk.h")
  end
end

RUBY_HEADERS = %w[
  ccan/check_type/check_type.h ccan/container_of/container_of.h ccan/list/list.h ccan/str/str.h
  gc/gc_impl.h gc/gc.h gc/extconf_base.rb
  darray.h
]
task :vendor_ruby_headers do
  RUBY_HEADERS.each do |file|
    Net::HTTP.start("raw.githubusercontent.com", 443, use_ssl: true) do |http|
      resp = http.get("ruby/ruby/refs/heads/master/#{file}")

      FileUtils.mkdir_p(File.dirname(file))
      open(file, "wb") do |file|
        file.write(resp.body)
      end
    end
  end
end
