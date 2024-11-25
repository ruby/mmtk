# frozen_string_literal: true

$LOAD_PATH << File.expand_path("tool", __dir__)

require 'fileutils'
require 'tempfile'
require 'net/http'
require 'digest'
require 'etc'
require 'mmtk_support'

task default: %i[compile install]

desc <<~DESC
  Install the MMTk GC shared object into the Shared GC dir for the currently
  running Ruby
DESC
task :install, :compile do
  install_mmtk
end

desc <<~DESC
  Build the MMTk GC implementation shared object
DESC
task :compile do
  MMTkSupport.new.build
end

desc <<~DESC
  Remove all generated build artifacts
DESC
task :clean do
  system("git clean -ffdx")
end
