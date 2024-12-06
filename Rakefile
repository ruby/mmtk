# frozen_string_literal: true

require "fileutils"
require "net/http"
require "rbconfig"
require "rake/extensiontask"

task default: [:compile, :install]

Rake::ExtensionTask.new do |ext|
  ext.name = "librubygc.mmtk"
  ext.ext_dir = "gc/mmtk"
  ext.lib_dir = "tmp/binaries"
end

task :install do
  FileUtils.mv(Dir.glob("tmp/binaries/*"), RbConfig::CONFIG["modular_gc_dir"])
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
