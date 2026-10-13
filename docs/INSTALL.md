# Installing OpenSGolf: the details

Every [release](https://github.com/crspytopgn/opensgolf/releases) has a package for each platform: unpack it and
double-click. It starts on the free base set. Have SimGolf? Choose **Use my SimGolf files...** on the title screen (or in
Preferences) and pick the folder SimGolf is installed in, or the CD's `Program_Files_(ENGLISH)` copied to your computer; the
game remembers it. It also finds a usual install by itself (`Program Files\Maxis\SimGolf`, `~/SimGolf`, a Wine install, or a
`SimGolf` folder next to the program; the full list is in [PLAYING.md](PLAYING.md#your-own-simgolf-files)). *Use the free
graphics* goes back. No Terminal and no options are needed (`--game DIR` remains for scripts).

* **Windows**: unzip `opensgolf-windows.zip` anywhere and double-click `opensgolf.exe` (no console window opens). Windows
  SmartScreen may warn that the program is unknown (it is not signed): *More info*, *Run anyway*. Saves go to
  `%APPDATA%\OpenSGolf`.
* **macOS** (Apple silicon and Intel): unzip `opensgolf-macos.zip` and move `OpenSGolf.app` to Applications. It is not
  notarised, so the first time right-click it and choose *Open* (or allow it under System Settings, Privacy & Security). Saves
  go to `~/Library/Application Support/OpenSGolf`.
* **Linux** (x86_64, and 64-bit ARM such as a Raspberry Pi 4/5: the `-arm64` files): the AppImage (allow running it as a program in its Properties, or `chmod +x`, then double-click it), or
  unpack the `.tar.gz` and run `opensgolf` in its folder (`opensgolf.desktop` and `opensgolf.png` in it are for a menu entry).
  The folder picker asks the desktop through the XDG portal (GNOME, KDE and most desktops have it). Sound goes through ALSA
  (PulseAudio and PipeWire provide it). Saves go to `~/.local/share/opensgolf`.
* **Android** (8.0 or newer; arm64, 32-bit arm and x86_64): download `opensgolf-android.apk` on the phone or
  tablet and open it. Android asks once to allow installing apps from the browser or file manager you used (*Install
  unknown apps*); allow it and install. The app is not in an app store and is signed with a development (debug) key, so a
  newer version may refuse to install over an older one: then uninstall the old one first (that removes its saves; copy them
  out of the app first if you want them, see below). The game plays in landscape, full screen, by touch
  ([docs/PLAYING.md, Touch](PLAYING.md#touch-phones-and-tablets)). **Your own copy of SimGolf on Android**: start the app
  once (it creates its folder), then copy your game's `Program_Files_(ENGLISH)` folder from a computer (USB cable, file
  transfer) into `Android/data/org.opensgolf.game/files/game/` on the device's storage, so that
  `.../files/game/Program_Files_(ENGLISH)/Flics` exists (copying the folder's contents straight into `game/` works too), and
  start the app again (the title screen's **Where do my SimGolf files go?** shows the folder). Saves, settings and packs are in the app's private storage (`/data/data/org.opensgolf.game/files`).
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
* **Browser**: nothing to install, see [Play in your browser](https://crspytopgn.github.io/opensgolf/); `opensgolf-web.zip` is the
  same page to host yourself.
* **From source**: see [Build and run](../README.md#build-and-run).

The application icon (a flag on a green and a ball) is drawn by the engine itself (`sgtool icon DIR`; `tools/make_icons.sh`
refreshes the Android launcher icons).

OpenSGolf is not affiliated with or endorsed by Electronic Arts or Firaxis Games. SimGolf and Sid Meier's are trademarks of
their owners and are named here only to say which game's files OpenSGolf can use.

