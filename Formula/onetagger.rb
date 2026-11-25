class Onetagger < Formula
  desc "Music library tagging application and command-line tools"
  homepage "https://github.com/fantuz/onetagger"
  url "https://github.com/fantuz/onetagger.git", tag: "v2.2.2"
  license "GPL-3.0-only"

  depends_on "node@24" => :build
  depends_on "pnpm" => :build
  depends_on "rustup" => :build
  depends_on :macos

  def install
    ENV.prepend_path "PATH", Formula["node@24"].opt_bin
    ENV["RUSTUP_HOME"] = buildpath/"rustup"
    ENV["CARGO_HOME"] = buildpath/"cargo-home"
    rustup = Formula["rustup"].opt_bin/"rustup"
    toolchain = (buildpath/"rust-toolchain.toml").read[/^channel = "([^"]+)"/, 1]

    system "pnpm", "--dir", "client", "install", "--frozen-lockfile", "--store-dir", buildpath/"pnpm-store"
    system "pnpm", "--dir", "client", "run", "build"
    system rustup, "toolchain", "install", toolchain, "--profile", "minimal"
    system rustup, "run", toolchain, "cargo", "build", "--locked", "--release", "-p", "onetagger", "-p", "onetagger-cli"
    bin.install "target/release/onetagger", "target/release/onetagger-cli"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/onetagger --version")
    assert_match version.to_s, shell_output("#{bin}/onetagger-cli --version")
  end
end
