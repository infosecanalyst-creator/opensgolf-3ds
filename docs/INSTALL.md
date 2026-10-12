# Installing OpenSGolf: the details

Every [release](https://github.com/crspytopgn/simgolf-native/releases) has a package for each platform. All of them start on
the free base set; to play with the original's art and sound, point them at your own copy of SimGolf as described.

* **Windows**: unzip `opensgolf-windows.zip` anywhere and run `opensgolf.exe`. Your own copy:
  `opensgolf.exe --game "C:\path\to\SimGolf\Program_Files_(ENGLISH)"` (a shortcut with that target works too), or copy the
  game's folder to `game\Program_Files_(ENGLISH)` next to where you start it. Windows SmartScreen may warn that the program is
  unknown (it is not signed): *More info*, *Run anyway*. Saves go to `%APPDATA%\OpenSGolf`.
* **macOS** (Apple silicon and Intel): unzip `opensgolf-macos.zip` and move `OpenSGolf.app` to
  Applications. It is not notarised, so the first time right-click it and choose *Open* (or allow it under System Settings,
  Privacy & Security). Your own copy: `/Applications/OpenSGolf.app/Contents/MacOS/opensgolf --game "/path/to/Program_Files_(ENGLISH)"`
  from a Terminal. Saves go to `~/Library/Application Support/OpenSGolf`.
* **Linux** (x86_64): either the AppImage (`chmod +x opensgolf-linux-x86_64.AppImage`, then run it; arguments
  such as `--game DIR` pass through), or unpack the `.tar.gz` and run `./opensgolf` in its folder (`opensgolf.desktop` and
  `opensgolf.png` in it are for a menu entry). Sound goes through ALSA (PulseAudio and PipeWire provide it). Saves go to
  `~/.local/share/opensgolf`.
* **Android** (8.0 or newer; arm64, 32-bit arm and x86_64): download `opensgolf-android.apk` on the phone or
  tablet and open it. Android asks once to allow installing apps from the browser or file manager you used (*Install
  unknown apps*); allow it and install. The app is not in an app store and is signed with a development (debug) key, so a
  newer version may refuse to install over an older one: then uninstall the old one first (that removes its saves; copy them
  out of the app first if you want them, see below). The game plays in landscape, full screen, by touch
  ([docs/PLAYING.md, Touch](PLAYING.md#touch-phones-and-tablets)). **Your own copy of SimGolf on Android**: start the app
  once (it creates its folder), then copy your game's `Program_Files_(ENGLISH)` folder from a computer (USB cable, file
  transfer) into `Android/data/org.opensgolf.game/files/game/` on the device's storage, so that
  `.../files/game/Program_Files_(ENGLISH)/Flics` exists (copying the folder's contents straight into `game/` works too), and
  start the app again. Saves, settings and packs are in the app's private storage (`/data/data/org.opensgolf.game/files`).
  Building the APK yourself: `docker run --rm -v "$(pwd)":/root/src -w /root/src notfl3/cargo-apk tools/android_build.sh`
  (see the script; CI builds a debug APK on every push, artifact `opensgolf-android-debug`).
* **Browser**: nothing to install, see [Play in your browser](https://crspytopgn.github.io/simgolf-native/); `opensgolf-web.zip` is the
  same page to host yourself.
* **From source**: see [Build and run](../README.md#build-and-run).

The application icon (a flag on a green and a ball) is drawn by the engine itself (`sgtool icon DIR`; `tools/make_icons.sh`
refreshes the Android launcher icons).

OpenSGolf is not affiliated with or endorsed by Electronic Arts or Firaxis Games. SimGolf and Sid Meier's are trademarks of
their owners and are named here only to say which game's files OpenSGolf can use.

