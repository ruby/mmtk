# frozen_string_literal: true

require_relative "helper"

module MMTk
  class TestMallocLimit < TestCase
    def test_malloc_growth_starts_gc
      assert_separately([], <<~RUBY)
        before = GC.count
        200.times { "x" * 1_000_000 }
        assert_operator(GC.count - before, :>=, 10)
      RUBY
    end

    def test_malloc_growth_does_not_start_gc_when_disabled
      assert_separately([], <<~RUBY)
        GC.disable
        before = GC.count
        200.times { "x" * 1_000_000 }
        assert_equal(before, GC.count)
      RUBY
    end

    %w(1048576 1024k 1024K 1m 1M).each do |limit|
      define_method(:"test_RUBY_GC_MALLOC_LIMIT_#{limit}") do
        env = { "RUBY_GC_MALLOC_LIMIT" => limit, "RUBY_GC_MALLOC_LIMIT_MAX" => limit }
        assert_separately([env], <<~RUBY)
          before = GC.count
          200.times { "x" * 1_000_000 }
          # A 1MiB limit runs a GC about every second string. The 10MiB
          # default would run about 20.
          assert_operator(GC.count - before, :>=, 60)
        RUBY
      end
    end

    %w(RUBY_GC_MALLOC_LIMIT RUBY_GC_MALLOC_LIMIT_MAX RUBY_GC_MALLOC_LIMIT_GROWTH_FACTOR).each do |var|
      define_method(:"test_invalid_#{var}") do
        assert_in_out_err(
          [{ var => "8 M" }, "-e", "p :ok"],
          "",
          [":ok"],
          ["[WARN] Ignoring invalid #{var}=8 M"],
          success: true
        )
      end
    end

    def test_infinite_RUBY_GC_MALLOC_LIMIT_GROWTH_FACTOR_is_rejected
      assert_in_out_err(
        [{ "RUBY_GC_MALLOC_LIMIT_GROWTH_FACTOR" => "inf" }, "-e", "p :ok"],
        "",
        [":ok"],
        ["[WARN] Ignoring invalid RUBY_GC_MALLOC_LIMIT_GROWTH_FACTOR=inf"],
        success: true
      )
    end

    def test_small_malloc_limit_during_boot
      env = { "RUBY_GC_MALLOC_LIMIT" => "16384", "RUBY_GC_MALLOC_LIMIT_MAX" => "16384" }
      assert_separately([env], <<~RUBY)
        assert_equal("ok", "ok".dup)
      RUBY
    end
  end
end
