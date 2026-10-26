# frozen_string_literal: true

require_relative "helper"

module MMTk
  class TestPinning < TestCase
    %w(Immix StickyImmix).each do |plan|
      define_method(:"test_string_buffer_held_by_c_frame_survives_gc_#{plan}") do
        assert_separately([{ "MMTK_PLAN" => plan }], <<~'RUBY')
          require "tempfile"

          Tempfile.create do |f|
            f.write("a\n" * 5000)
            f.flush

            s = File.read(f.path)
            moving_gc_count = GC.stat(:moving_gc_count)
            count = 0
            s.scan(/a/) do
              count += 1
              GC.start if count % 250 == 0
            end

            assert_equal(5000, count)
            assert_operator(GC.stat(:moving_gc_count), :>, moving_gc_count)
          end
        RUBY
      end
    end
  end
end
