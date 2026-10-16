package TARGET_PACKAGE_NAME;

import javax.microedition.khronos.egl.EGLConfig;
import javax.microedition.khronos.opengles.GL10;

import android.app.Activity;
import android.app.PendingIntent;
import android.os.Bundle;
import android.os.Build;
import android.util.Log;

import android.view.View;
import android.view.Surface;
import android.view.Window;
import android.view.WindowInsets;
import android.view.WindowManager.LayoutParams;
import android.view.SurfaceView;
import android.view.SurfaceHolder;
import android.view.MotionEvent;
import android.view.KeyEvent;
import android.view.InputDevice;
import android.view.inputmethod.InputMethodManager;

import android.content.Context;
import android.content.Intent;
import android.content.res.Configuration;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.pm.PackageInstaller;

import android.database.Cursor;
import android.net.Uri;
import android.provider.DocumentsContract;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;

import android.graphics.Color;
import android.graphics.Insets;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.EditorInfo;
import android.widget.LinearLayout;

import quad_native.QuadNative;

// note: //% is a special miniquad's pre-processor for plugins
// when there are no plugins - //% whatever will be replaced to an empty string
// before compiling

//% IMPORTS

class QuadSurface
    extends
        SurfaceView
    implements
        View.OnTouchListener,
        View.OnKeyListener,
        SurfaceHolder.Callback {

    public QuadSurface(Context context){
        super(context);
        getHolder().addCallback(this);

        setFocusable(true);
        setFocusableInTouchMode(true);
        requestFocus();
        setOnTouchListener(this);
        setOnKeyListener(this);
    }

    @Override
    public void surfaceCreated(SurfaceHolder holder) {
        Log.i("SAPP", "surfaceCreated");
        Surface surface = holder.getSurface();
        QuadNative.surfaceOnSurfaceCreated(surface);
    }

    @Override
    public void surfaceDestroyed(SurfaceHolder holder) {
        Log.i("SAPP", "surfaceDestroyed");
        Surface surface = holder.getSurface();
        QuadNative.surfaceOnSurfaceDestroyed(surface);
    }

    @Override
    public void surfaceChanged(SurfaceHolder holder,
                               int format,
                               int width,
                               int height) {
        Log.i("SAPP", "surfaceChanged");
        Surface surface = holder.getSurface();
        QuadNative.surfaceOnSurfaceChanged(surface, width, height);

    }

    @Override
    public boolean onTouch(View v, MotionEvent event) {
        int pointerCount = event.getPointerCount();
        int action = event.getActionMasked();

        switch(action) {
        case MotionEvent.ACTION_MOVE: {
            for (int i = 0; i < pointerCount; i++) {
                final int id = event.getPointerId(i);
                final float x = event.getX(i);
                final float y = event.getY(i);
                QuadNative.surfaceOnTouch(id, 0, x, y);
            }
            break;
        }
        case MotionEvent.ACTION_UP: {
            final int id = event.getPointerId(0);
            final float x = event.getX(0);
            final float y = event.getY(0);
            QuadNative.surfaceOnTouch(id, 1, x, y);
            break;
        }
        case MotionEvent.ACTION_DOWN: {
            final int id = event.getPointerId(0);
            final float x = event.getX(0);
            final float y = event.getY(0);
            QuadNative.surfaceOnTouch(id, 2, x, y);
            break;
        }
        case MotionEvent.ACTION_POINTER_UP: {
            final int pointerIndex = event.getActionIndex();
            final int id = event.getPointerId(pointerIndex);
            final float x = event.getX(pointerIndex);
            final float y = event.getY(pointerIndex);
            QuadNative.surfaceOnTouch(id, 1, x, y);
            break;
        }
        case MotionEvent.ACTION_POINTER_DOWN: {
            final int pointerIndex = event.getActionIndex();
            final int id = event.getPointerId(pointerIndex);
            final float x = event.getX(pointerIndex);
            final float y = event.getY(pointerIndex);
            QuadNative.surfaceOnTouch(id, 2, x, y);
            break;
        }
        case MotionEvent.ACTION_CANCEL: {
            for (int i = 0; i < pointerCount; i++) {
                final int id = event.getPointerId(i);
                final float x = event.getX(i);
                final float y = event.getY(i);
                QuadNative.surfaceOnTouch(id, 3, x, y);
            }
            break;
        }
        default:
            break;
        }

        return true;
    }

    // docs says getCharacters are deprecated
    // but somehow on non-latyn input all keyCode and all the relevant fields in the KeyEvent are zeros
    // and only getCharacters has some usefull data
    @SuppressWarnings("deprecation")
    @Override
    public boolean onKey(View v, int keyCode, KeyEvent event) {
        // a game controller's face, shoulder and stick buttons go to the game's controller code (gamepad.rs) as the bits
        // of the W3C standard gamepad; the d-pad stays the arrow keys
        int bit = padBit(keyCode);
        if (bit >= 0) {
            if (event.getAction() == KeyEvent.ACTION_DOWN) {
                padButtons |= 1 << bit;
            } else if (event.getAction() == KeyEvent.ACTION_UP) {
                padButtons &= ~(1 << bit);
            }
            sendPad();
            return true;
        }
        if (event.getAction() == KeyEvent.ACTION_DOWN && keyCode != 0) {
            QuadNative.surfaceOnKeyDown(keyCode);
        }

        if (event.getAction() == KeyEvent.ACTION_UP && keyCode != 0) {
            QuadNative.surfaceOnKeyUp(keyCode);
        }

        if (event.getAction() == KeyEvent.ACTION_UP || event.getAction() == KeyEvent.ACTION_MULTIPLE) {
            int character = event.getUnicodeChar();
            if (character == 0) {
                String characters = event.getCharacters();
                if (characters != null && !characters.isEmpty()) {
                    character = characters.charAt(0);
                }
            }

            if (character != 0) {
                QuadNative.surfaceOnCharacter(character);
            }
        }

        return true;
    }

    private int padButtons = 0;
    private float padLx, padLy, padRx, padRy;

    private static int padBit(int keyCode) {
        switch (keyCode) {
        case KeyEvent.KEYCODE_BUTTON_A: return 0;
        case KeyEvent.KEYCODE_BUTTON_B: return 1;
        case KeyEvent.KEYCODE_BUTTON_X: return 2;
        case KeyEvent.KEYCODE_BUTTON_Y: return 3;
        case KeyEvent.KEYCODE_BUTTON_L1: return 4;
        case KeyEvent.KEYCODE_BUTTON_R1: return 5;
        case KeyEvent.KEYCODE_BUTTON_L2: return 6;
        case KeyEvent.KEYCODE_BUTTON_R2: return 7;
        case KeyEvent.KEYCODE_BUTTON_SELECT: return 8;
        case KeyEvent.KEYCODE_BUTTON_START: return 9;
        case KeyEvent.KEYCODE_BUTTON_THUMBL: return 10;
        case KeyEvent.KEYCODE_BUTTON_THUMBR: return 11;
        default: return -1;
        }
    }

    private void sendPad() {
        QuadNative.surfaceOnGamepad(padButtons, padLx, padLy, padRx, padRy);
    }

    // a controller's sticks, analogue triggers and hat (a d-pad some controllers report as axes)
    @Override
    public boolean onGenericMotionEvent(MotionEvent e) {
        if ((e.getSource() & InputDevice.SOURCE_JOYSTICK) == InputDevice.SOURCE_JOYSTICK
                && e.getAction() == MotionEvent.ACTION_MOVE) {
            padLx = e.getAxisValue(MotionEvent.AXIS_X);
            padLy = e.getAxisValue(MotionEvent.AXIS_Y);
            padRx = e.getAxisValue(MotionEvent.AXIS_Z);
            padRy = e.getAxisValue(MotionEvent.AXIS_RZ);
            float lt = Math.max(e.getAxisValue(MotionEvent.AXIS_LTRIGGER), e.getAxisValue(MotionEvent.AXIS_BRAKE));
            float rt = Math.max(e.getAxisValue(MotionEvent.AXIS_RTRIGGER), e.getAxisValue(MotionEvent.AXIS_GAS));
            float hx = e.getAxisValue(MotionEvent.AXIS_HAT_X), hy = e.getAxisValue(MotionEvent.AXIS_HAT_Y);
            // analogue triggers and the hat set the bits their buttons would (LT 6, RT 7, d-pad 12..15)
            int axisBits = (1 << 6) | (1 << 7) | (0xf << 12);
            int b = padButtons & ~axisBits;
            if (lt > 0.5f) b |= 1 << 6;
            if (rt > 0.5f) b |= 1 << 7;
            if (hy < -0.5f) b |= 1 << 12;
            if (hy > 0.5f) b |= 1 << 13;
            if (hx < -0.5f) b |= 1 << 14;
            if (hx > 0.5f) b |= 1 << 15;
            padButtons = b;
            sendPad();
            return true;
        }
        return super.onGenericMotionEvent(e);
    }

    // There is an Android bug when screen is in landscape,
    // the keyboard inset height is reported as 0.
    // This code is a workaround which fixes the bug.
    // See https://groups.google.com/g/android-developers/c/50XcWooqk7I
    // For some reason it only works if placed here and not in the parent layout.
    @Override
    public InputConnection onCreateInputConnection(EditorInfo outAttrs) {
        //% QUAD_SURFACE_ON_CREATE_INPUT_CONNECTION

        InputConnection connection = super.onCreateInputConnection(outAttrs);
        outAttrs.imeOptions |= EditorInfo.IME_FLAG_NO_FULLSCREEN;
        return connection;
    }

    public Surface getNativeSurface() {
        return getHolder().getSurface();
    }
}

class ResizingLayout
    extends
        LinearLayout
    implements
        View.OnApplyWindowInsetsListener {
    //% RESIZING_LAYOUT_BODY

    public ResizingLayout(MainActivity activity){
        super(activity);
        // When viewing in landscape mode with keyboard shown, there are
        // gaps on both sides so we fill the negative space with black.
        setBackgroundColor(Color.BLACK);
        setOnApplyWindowInsetsListener(this);

        //% RESIZING_LAYOUT_CONSTRUCTOR
    }

    @Override
    public WindowInsets onApplyWindowInsets(View v, WindowInsets insets) {
        // This handler provides a default impl which resizes content when the
        // IME is shown or hidden.
        //
        // However this will lead to flickers in your app as the content is
        // resized before you have a chance to redraw it.
        //
        // The workaround for now is:
        // * Get IME and system insets and send them to your app.
        // * Notify your app to draw and apply the insets to fit.
        // * Disable this function by returning early.

        //% RESIZING_LAYOUT_ON_APPLY_WINDOW_INSETS

        if (Build.VERSION.SDK_INT >= 30) {
            Insets imeInsets = insets.getInsets(WindowInsets.Type.ime());
            Insets sysInsets = insets.getInsets(WindowInsets.Type.systemBars());

            // When IME is visible then we dont need bottom inset
            int bottomPadding = sysInsets.bottom;
            if (imeInsets.bottom > 0) {
                bottomPadding = imeInsets.bottom;
            }

            // The sys insets change when orientation changes and sys bars
            // change position.
            v.setPadding(
                sysInsets.left,
                sysInsets.top,
                sysInsets.right,
                bottomPadding
            );
        }
        return insets;
    }
}

public class MainActivity extends Activity {
    //% MAIN_ACTIVITY_BODY

    private QuadSurface view;

    // OpenSGolf: the game's library by its own name. cargo-quad-apk puts the last part of the package name here
    // (org.opensgolf.game: "game"), but the library is named after the crate (libopensgolf.so), so the app closed at
    // once with "dlopen failed: library libgame.so not found" (README-OPENSGOLF.md).
    static {
        System.loadLibrary("opensgolf");
    }

    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        this.requestWindowFeature(Window.FEATURE_NO_TITLE);

        view = new QuadSurface(this);
        // Put it inside a parent layout which can resize it using padding
        ResizingLayout layout = new ResizingLayout(this);
        layout.addView(view);
        setContentView(layout);

        QuadNative.activityOnCreate(this);

        //% MAIN_ACTIVITY_ON_CREATE
    }

    @Override
    protected void onResume() {
        super.onResume();
        QuadNative.activityOnResume();

        //% MAIN_ACTIVITY_ON_RESUME
    }

    @Override
    public void onBackPressed() {
        Log.w("SAPP", "onBackPressed");

        // TODO: here is the place to handle request_quit/order_quit/cancel_quit

        super.onBackPressed();
    }

    @Override
    protected void onStop() {
        super.onStop();
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();

        QuadNative.activityOnDestroy();
    }

    @Override
    protected void onPause() {
        super.onPause();
        QuadNative.activityOnPause();

        //% MAIN_ACTIVITY_ON_PAUSE
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        //% MAIN_ACTIVITY_ON_ACTIVITY_RESULT

        if (requestCode == PICK_ZIP || requestCode == PICK_TREE) {
            onGameFilesPicked(requestCode == PICK_TREE, resultCode, data);
        }
    }

    // OpenSGolf: "Use my SimGolf files..." (crates/simgolf/src/android.rs). Since Android 11 a file manager cannot reach
    // Android/data/org.opensgolf.game, so the game asks Android's own picker for a .zip of the SimGolf folder
    // (ACTION_OPEN_DOCUMENT) or the folder itself (ACTION_OPEN_DOCUMENT_TREE), and what the player picked is copied here,
    // on a thread, into the app's own folders: the game reads importState() each frame and unpacks the copy (import.rs).
    // The picker grants access to what was picked: no storage permission is needed.
    private static final int PICK_ZIP = 7301;
    private static final int PICK_TREE = 7302;
    // static: the copy outlives an activity that Android makes again while its picker is in front
    private static volatile String importState = "";
    private static String importDest;
    private static long importFiles, importBytes;

    public String importState() {
        return importState;
    }

    public void pickGameFiles(final boolean folder, final String dest) {
        importState = "picking";
        importDest = dest;
        runOnUiThread(new Runnable() {
                @Override
                public void run() {
                    Intent i;
                    if (folder) {
                        i = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
                    } else {
                        i = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                        i.addCategory(Intent.CATEGORY_OPENABLE);
                        i.setType("*/*");
                        i.putExtra(Intent.EXTRA_MIME_TYPES, new String[] {
                            "application/zip", "application/x-zip-compressed", "application/x-zip", "application/octet-stream"
                        });
                    }
                    try {
                        startActivityForResult(i, folder ? PICK_TREE : PICK_ZIP);
                    } catch (Exception e) {
                        importState = "failed:" + e;
                    }
                }
            });
    }

    private void onGameFilesPicked(final boolean tree, int resultCode, Intent data) {
        final Uri uri = data == null ? null : data.getData();
        if (resultCode != RESULT_OK || uri == null) {
            importState = "cancelled";
            return;
        }
        Log.i("OpenSGolf", "picked " + uri);
        importFiles = 0;
        importBytes = 0;
        importState = "copying:0:0";
        new Thread(new Runnable() {
                @Override
                public void run() {
                    try {
                        File dest = new File(importDest);
                        deleteTree(dest);
                        if (tree) {
                            dest.mkdirs();
                            copyTree(uri, DocumentsContract.getTreeDocumentId(uri), dest);
                            importState = "dir:" + dest.getAbsolutePath();
                        } else {
                            dest.getParentFile().mkdirs();
                            copyFile(uri, dest);
                            importState = "zip:" + dest.getAbsolutePath();
                        }
                    } catch (Throwable e) {
                        Log.e("OpenSGolf", "copying the picked files failed", e);
                        importState = "failed:" + e.getMessage();
                    }
                }
            }, "opensgolf-copy").start();
    }

    // What the game does not need (as import.rs): a Mac's resource forks, videos, Windows programs.
    private static boolean skipName(String low) {
        return low.equals("__macosx") || low.startsWith("._") || low.equals(".ds_store") || low.endsWith(".bik")
            || low.endsWith(".exe") || low.endsWith(".dll") || low.endsWith(".lib");
    }

    private void copyTree(Uri tree, String docId, File dest) throws IOException {
        Uri children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, docId);
        String[] cols = { DocumentsContract.Document.COLUMN_DOCUMENT_ID, DocumentsContract.Document.COLUMN_DISPLAY_NAME,
                          DocumentsContract.Document.COLUMN_MIME_TYPE };
        Cursor c = getContentResolver().query(children, cols, null, null, null);
        if (c == null) {
            throw new IOException("cannot list " + docId);
        }
        try {
            while (c.moveToNext()) {
                String id = c.getString(0), name = c.getString(1), mime = c.getString(2);
                if (name == null || name.contains("/") || name.equals(".") || name.equals("..")
                    || skipName(name.toLowerCase())) {
                    continue;
                }
                File out = new File(dest, name);
                if (DocumentsContract.Document.MIME_TYPE_DIR.equals(mime)) {
                    out.mkdirs();
                    copyTree(tree, id, out);
                } else {
                    copyFile(DocumentsContract.buildDocumentUriUsingTree(tree, id), out);
                    importFiles++;
                }
            }
        } finally {
            c.close();
        }
    }

    private void copyFile(Uri from, File to) throws IOException {
        InputStream in = getContentResolver().openInputStream(from);
        if (in == null) {
            throw new IOException("cannot open " + from);
        }
        OutputStream out = new FileOutputStream(to);
        try {
            byte[] buf = new byte[1 << 16];
            int n;
            while ((n = in.read(buf)) > 0) {
                out.write(buf, 0, n);
                importBytes += n;
                importState = "copying:" + importFiles + ":" + importBytes;
            }
        } finally {
            in.close();
            out.close();
        }
    }

    private static void deleteTree(File f) {
        File[] kids = f.listFiles();
        if (kids != null) {
            for (File k : kids) {
                deleteTree(k);
            }
        }
        f.delete();
    }

    // OpenSGolf: "Update now" (crates/simgolf/src/android.rs install_apk, autoupdate.rs). The game has downloaded the new
    // APK and checked its signature and checksum; it is written into a PackageInstaller session (so no file:// or
    // content:// address has to be shared, and no FileProvider is needed), and Android asks the player to confirm: the
    // session's status comes back to this activity (onNewIntent), which shows Android's box. Needs the
    // REQUEST_INSTALL_PACKAGES permission (crates/simgolf/Cargo.toml); Android asks the player once to allow OpenSGolf to
    // install apps, and installs an update only when it is signed with the same key as the installed app.
    private static final String INSTALL_STATUS = "org.opensgolf.game.INSTALL_STATUS";
    private static volatile String installState = "";

    public String installState() {
        return installState;
    }

    public void installApk(final String path) {
        installState = "installing";
        new Thread(new Runnable() {
                @Override
                public void run() {
                    try {
                        PackageInstaller pi = getPackageManager().getPackageInstaller();
                        PackageInstaller.SessionParams params =
                            new PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL);
                        int id = pi.createSession(params);
                        PackageInstaller.Session session = pi.openSession(id);
                        try {
                            File apk = new File(path);
                            InputStream in = new FileInputStream(apk);
                            OutputStream out = session.openWrite("opensgolf.apk", 0, apk.length());
                            try {
                                byte[] buf = new byte[1 << 16];
                                int n;
                                while ((n = in.read(buf)) > 0) {
                                    out.write(buf, 0, n);
                                }
                                session.fsync(out);
                            } finally {
                                in.close();
                                out.close();
                            }
                            Intent i = new Intent(MainActivity.this, MainActivity.class);
                            i.setAction(INSTALL_STATUS);
                            i.addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP);
                            // Android 12 wants the mutability said; the installer fills in the status, so mutable
                            int flags = PendingIntent.FLAG_UPDATE_CURRENT | (Build.VERSION.SDK_INT >= 31 ? 0x02000000 : 0);
                            PendingIntent pending = PendingIntent.getActivity(MainActivity.this, 7303, i, flags);
                            session.commit(pending.getIntentSender());
                        } finally {
                            session.close();
                        }
                        installState = "waiting";
                    } catch (Throwable e) {
                        Log.e("OpenSGolf", "installing the update failed", e);
                        installState = "failed:" + e.getMessage();
                    }
                }
            }, "opensgolf-install").start();
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        if (intent == null || !INSTALL_STATUS.equals(intent.getAction())) {
            return;
        }
        int status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE);
        if (status == PackageInstaller.STATUS_PENDING_USER_ACTION) {
            Intent confirm = (Intent) intent.getParcelableExtra(Intent.EXTRA_INTENT);
            try {
                if (confirm != null) {
                    startActivity(confirm);
                }
                installState = "confirm";
            } catch (Exception e) {
                installState = "failed:" + e;
            }
        } else if (status == PackageInstaller.STATUS_SUCCESS) {
            installState = "done";
        } else {
            String msg = intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE);
            installState = "failed:" + (msg != null ? msg : "status " + status);
        }
    }

    // OpenSGolf: starts the app again (the player's files load from the start on a new run, as on a desktop).
    public void restartApp() {
        runOnUiThread(new Runnable() {
                @Override
                public void run() {
                    startActivity(Intent.makeRestartActivityTask(getComponentName()));
                    android.os.Process.killProcess(android.os.Process.myPid());
                }
            });
    }

    public void setFullScreen(final boolean fullscreen) {
        runOnUiThread(new Runnable() {
                @Override
                public void run() {
                    View decorView = getWindow().getDecorView();

                    if (fullscreen) {
                        getWindow().setFlags(LayoutParams.FLAG_LAYOUT_NO_LIMITS, LayoutParams.FLAG_LAYOUT_NO_LIMITS);
                        if (Build.VERSION.SDK_INT >= 28) {
                            getWindow().getAttributes().layoutInDisplayCutoutMode = LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
                        }
                        if (Build.VERSION.SDK_INT >= 30) {
                            getWindow().setDecorFitsSystemWindows(false);
                        } else {
                            int uiOptions = View.SYSTEM_UI_FLAG_HIDE_NAVIGATION | View.SYSTEM_UI_FLAG_FULLSCREEN | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY;
                            decorView.setSystemUiVisibility(uiOptions);
                        }
                    }
                    else {
                        if (Build.VERSION.SDK_INT >= 30) {
                            getWindow().setDecorFitsSystemWindows(true);
                        } else {
                          decorView.setSystemUiVisibility(0);
                        }

                    }
                }
            });
    }

    public void showKeyboard(final boolean show) {
        runOnUiThread(new Runnable() {
                @Override
                public void run() {
                    if (show) {
                        InputMethodManager imm = (InputMethodManager)getSystemService(Context.INPUT_METHOD_SERVICE);
                        imm.showSoftInput(view, 0);
                    } else {
                        InputMethodManager imm = (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);
                        imm.hideSoftInputFromWindow(view.getWindowToken(),0);
                    }
                }
            });
    }

    public String getClipboardText() {
        ClipboardManager clipboard = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);

        if (!clipboard.hasPrimaryClip())
            return null;

        ClipData primaryClip = clipboard.getPrimaryClip();
        if (primaryClip == null || primaryClip.getItemCount() < 1)
            return null;

        CharSequence clipData = clipboard.getPrimaryClip().getItemAt(0).getText();
        if (clipData == null) {
            return null;
        }

        return clipData.toString();
    }
    public void setClipboardText(String text) {
        ClipboardManager clipboard = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
        ClipData clip = ClipData.newPlainText("label", text);
        clipboard.setPrimaryClip(clip);
    }
}

