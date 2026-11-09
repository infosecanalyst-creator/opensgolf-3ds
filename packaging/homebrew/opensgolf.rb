# DRAFT Homebrew cask for OpenSGolf (packaging/README.md, "Homebrew"). @VERSION@ is the release without its "v" (0.5.6),
# @SHA256@ the SHA-256 of that release's opensgolf-macos.zip (it is in the release's SHA256SUMS).
cask "opensgolf" do
  version "@VERSION@"
  sha256 "@SHA256@"

  url "https://github.com/crspytopgn/opensgolf/releases/download/v#{version}/opensgolf-macos.zip"
  name "OpenSGolf"
  desc "Open source engine for the golf course building game SimGolf"
  homepage "https://github.com/crspytopgn/opensgolf"

  livecheck do
    url :url
    strategy :github_latest
  end

  # the game can update itself (title menu: Check for updates, Update now)
  auto_updates true
  # one universal app for Apple silicon and Intel; packaging/macos/Info.plist LSMinimumSystemVersion
  depends_on macos: ">= :big_sur"

  # the zip holds a folder opensgolf-macos with the app and the documents (.github/workflows/release.yml)
  app "opensgolf-macos/OpenSGolf.app"

  zap trash: "~/Library/Application Support/OpenSGolf"
end
