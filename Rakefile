# frozen_string_literal: true

require "fileutils"
require "net/http"

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
