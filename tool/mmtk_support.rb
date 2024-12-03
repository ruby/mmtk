class MMTkSupport
  def build
    clone_ruby_repo
    cp_mmtk_gc
    configure_ruby
    make_ruby
    make_shared_gc_mmtk
  end

  private

  def clone_ruby_repo
    FileUtils.mkdir_p("res/ruby")
    unless Dir.exist?("res/ruby/.git")
      system("git clone -b mvh-modgc-ci --single-branch --depth 1 https://github.com/shopify/ruby.git res/ruby") or
        raise "Failed to clone Ruby repository"
    end
  end

  def cp_mmtk_gc
    FileUtils.rm_rf("res/ruby/gc/mmtk")
    FileUtils.cp_r("#{FileUtils.pwd}/gc/mmtk", "res/ruby/gc/mmtk")
  end

  def configure_ruby
    base_dir = FileUtils.pwd

    unless File.exist?("res/ruby/Makefile")
      FileUtils.chdir("res/ruby") do
        system("./autogen.sh")
        system(<<~EOF)
          ./configure --prefix=#{FileUtils.pwd}/install \
            --disable-install-doc \
            --with-shared-gc=#{base_dir}/mod_gc
        EOF
      end
    end
  end

  def make_ruby
    unless File.exist?("res/ruby/ruby")
      FileUtils.chdir("res/ruby") do
        system("make -j#{Etc.nprocessors}")
      end
    end
  end

  def make_shared_gc_mmtk
    FileUtils.chdir("res/ruby") do
      system("make shared-gc SHARED_GC=mmtk")
    end
  end
end
