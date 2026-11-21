// selftest_port.h - SimGolf's additions to the shared selftest.c (3ds-test-suite hwtest/selftest).
// selftest.c includes this file twice: ST_PORT_CONFIG (settings) and ST_PORT_CODE (extra code).
#ifdef ST_PORT_CONFIG
// SimGolf quits by itself when the script is done: aptMainLoop returns false, the game saves and
// returns. Being ended by the watchdog instead brought up the system's "an error has occurred"
// screen (the preloader thread on core 2 was still running), so it gets 30 s first.
#define ST_APT_QUIT 1
#define ST_DONE_GRACE_MS 30000
#endif
#ifdef ST_PORT_CODE
// Always-on crash restart (3ds-test-suite hwtest/restartguard): compiled into the selftest object
// so it is always linked.
#include "restartguard.c"
#endif
