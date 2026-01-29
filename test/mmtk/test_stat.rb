# frozen_string_literal: true

require_relative "helper"

module MMTk
  class TestStat < TestCase
    def test_weak_references_count
      assert_operator(GC.stat(:weak_references_count), :>, 0)

      EnvUtil.without_gc do
        before = GC.stat(:weak_references_count)
        ObjectSpace::WeakMap.new
        assert_operator(GC.stat(:weak_references_count), :>, before)
      end
    end
  end
end
