# Installing OpenSGolf: the details

Every [release](https://github.com/crspytopgn/opensgolf/releases) has a package for each platform: unpack it and
double-click. It starts on the free base set. Have SimGolf? Choose **Use my SimGolf files...** on the title screen (or in
Preferences) and pick either **a .zip** of your SimGolf folder (the same .zip the browser version takes) or **the folder**
SimGolf is installed in (or the CD's `Program_Files_(ENGLISH)` copied to your computer, or any folder above it); the game
remembers it. A .zip is unpacked into the game's own folder (`game` in the save folder), leaving out the videos, the
Windows programs and a Mac's `__MACOSX` files; afterwards the .zip is no longer needed. It also finds a usual install by itself (`Program Files\Maxis\SimGolf`, `~/SimGolf`, a Wine install, or a
`SimGolf` folder next to the program; the full list is in [PLAYING.md](PLAYING.md#your-own-simgolf-files)). *Use the free
graphics* goes back. No Terminal and no options are needed (`--game DIR` remains for scripts).

* **Windows**: unzip `opensgolf-windows.zip` anywhere and double-click `opensgolf.exe` (no console window opens). Windows
  SmartScreen may warn that the program is unknown (it is not signed): *More info*, *Run anyway*. Saves go to
  `%APPDATA%\OpenSGolf`.
* **macOS** (Apple silicon and Intel): unzip `opensgolf-macos.zip` and move `OpenSGolf.app` to Applications. It is not
  notarised, so the first time right-click it and choose *Open*; on macOS 15 and newer that no longer offers *Open*:
  double-click it once, then System Settings → Privacy & Security → *Open Anyway* (near the bottom), and confirm. If macOS
  says the app "is damaged and can't be opened", that is the download's quarantine flag, not damage: in Terminal run
  `xattr -dr com.apple.quarantine /Applications/OpenSGolf.app` once. Saves go to `~/Library/Application Support/OpenSGolf`.
  **Your SimGolf files on a Mac**: choose **Use my SimGolf files...**, then **A .zip file** and pick the .zip (a folder
  picker on a Mac greys out .zip files, which is why the game asks which one first), or **A folder** and pick the unpacked
  SimGolf folder. Either way the files are copied into `~/Library/Application Support/OpenSGolf/game`, because macOS's
  privacy rules may stop the game from reading Desktop, Documents or Downloads on a later start. When macOS asks whether
  OpenSGolf may access files in one of those folders, choose *Allow*; if you chose *Don't Allow* before, turn OpenSGolf on
  under System Settings → Privacy & Security → Files and Folders (the game says so when it cannot read a folder). A right
  click is Ctrl+click or a two-finger click. No sound? The game plays on the Mac's current output device and follows a
  change (headphones, AirPods, another output in Control Centre) within two seconds; if it stays silent, start it from
  Terminal with `/Applications/OpenSGolf.app/Contents/MacOS/opensgolf` and look for the lines starting with `sound:`.
* **Linux** (x86_64, and 64-bit ARM such as a Raspberry Pi 4/5: the `-arm64` files): the AppImage (allow running it as a program in its Properties, or `chmod +x`, then double-click it), or
  unpack the `.tar.gz` and run `opensgolf` in its folder (`opensgolf.desktop` and `opensgolf.png` in it are for a menu entry).
  The folder picker asks the desktop through the XDG portal (GNOME, KDE and most desktops have it). Sound goes through ALSA
  (PulseAudio and PipeWire provide it). Saves go to `~/.local/share/opensgolf`.
* **Android** (8.0 or newer; arm64, 32-bit arm and x86_64): download `opensgolf-android.apk` on the phone or
  tablet and open it. Android asks once to allow installing apps from the browser or file manager you used (*Install
  unknown apps*); allow it and install. The app is not in an app store and is signed with a development (debug) key, so a
  newer version may refuse to install over an older one: then uninstall the old one first (that removes its saves; copy them
  out of the app first if you want them, see below). The game plays in landscape, full screen, by touch
  ([docs/PLAYING.md, Touch](PLAYING.md#touch-phones-and-tablets)). **Your own copy of SimGolf on Android**: put a .zip of
  your SimGolf folder (or the folder itself) anywhere on the phone (Downloads, a USB stick, Google Drive...), tap **Use my
  SimGolf files...** on the title screen, choose **A .zip file** or **A folder**, and pick it in Android's file picker
  (for a folder, Android asks to *Allow* OpenSGolf to see it; it cannot pick the top of the storage or the Download folder
  itself, so put the SimGolf folder in a folder of its own). The game copies the files into its own storage, showing how far
  it is, and starts again with them; afterwards the .zip or folder you picked is no longer needed. A .zip is the quickest way
  (Android's folder picker copies file by file). The files end up in `Android/data/org.opensgolf.game/files/game/`; copying
  them there by hand also still works, but Android 11 and newer hide that folder from file managers and USB (only `adb push`
  reaches it). Saves, settings and packs are in the app's private storage (`/data/data/org.opensgolf.game/files`).
  **If the app closes right away**: version 0.1.0's APK closed the moment it was opened on every phone (its library could not
  be loaded: the C++ runtime of the sound output was missing from it); install a newer one. Should the app still close or
  freeze, the next start shows a box saying so, whose **Copy the report** button copies what went wrong (paste it into an
  issue or a message). It also writes what went wrong to `Android/data/org.opensgolf.game/files/crash.txt` on the device's storage (copy it to
  a computer over USB, or open it with a file manager that shows `Android/data`) and to the system log under the tag
  `OpenSGolf`: with USB debugging on (Settings → About phone → tap *Build number* seven times, then Developer options → *USB
  debugging*) and the phone on a computer with the Android platform tools, `adb logcat -s OpenSGolf` shows the game's
  messages while it starts (`adb logcat -d > log.txt` saves the whole log, which also has Android's own reason when the app
  cannot even start). Please attach `crash.txt` or that log to a report on the issue tracker, with the phone's model and
  Android version.
  Building the APK yourself: `docker run --rm -v "$(pwd)":/root/src -w /root/src notfl3/cargo-apk tools/android_build.sh`
  (see the script; CI builds a debug APK on every push, artifact `opensgolf-android-debug`).
* **If the game crashes** on Windows, macOS or Linux, it writes what went wrong to `crash.txt` in its save folder:
  `%APPDATA%\OpenSGolf` on Windows, `~/Library/Application Support/OpenSGolf` on a Mac (Finder: Go → Go to Folder…), and
  `~/.local/share/opensgolf` on Linux. Please attach it to a report on the issue tracker, with what you clicked just before.
* **Updating**: on the title menu, **Check for updates** says whether a newer version is out, and **Update now**
  downloads it, checks its signature and installs it over this copy, then starts it (details and what is sent in
  [PLAYING.md, The title menu](PLAYING.md#the-title-menu)). It works for the packages as they come: the Windows and
  Linux `.tar.gz` folder (the game must be allowed to write to its folder), the AppImage, `OpenSGolf.app` (where you
  may change it: in `/Applications` as an administrator, or anywhere in your home folder) and the Android app (Android
  asks you to confirm; it installs over the old app only when both are signed with the same key). Your saves and
  settings are elsewhere and stay as they are. Where it cannot update itself (a build from the sources, a folder it may
  not write to, a release without a valid signature) it says why and opens the download page instead: download the new
  package and install it as the first time. The *Check for updates when the game starts* extra (Preferences →
  OpenSGolf extras → Interface, off by default) asks once at each start.
* **No sound?** The game writes what it found and what it plays on to `sound.txt` in the same save folder (20 seconds after
  it starts, then every two minutes; in the browser the same lines go to the console, F12). Please attach it to a report.
* **Browser**: nothing to install, see [Play in your browser](https://crspytopgn.github.io/opensgolf/); `opensgolf-web.zip` is the
  same page to host yourself.
* **From source**: see [Build and run](../README.md#build-and-run).

The application icon (a flag on a green and a ball) is drawn by the engine itself (`sgtool icon DIR`; `tools/make_icons.sh`
refreshes the Android launcher icons).

OpenSGolf is not affiliated with or endorsed by Electronic Arts or Firaxis Games. SimGolf and Sid Meier's are trademarks of
their owners and are named here only to say which game's files OpenSGolf can use.

