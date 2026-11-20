// Heap set-up for a 3DS homebrew app (replaces libctru's __system_allocateHeaps).
// Two fixes from real hardware (Nov 13-15 2026):
//  - wait (up to 3 s) for the previous app's memory to be released, until the free amount settles;
//  - leave a few MB uncommitted. With every byte committed, the next program the Homebrew
//    loader starts after this one quits fails to load (D86007F3 in the "loader" process, or a
//    svcBreak in the Homebrew Menu's own heap set-up).
#include <3ds.h>
#include <3ds/allocator/mappable.h>

extern char* fake_heap_start;
extern char* fake_heap_end;
extern u32 __ctru_heap;
extern u32 __ctru_heap_size;
extern u32 __ctru_linear_heap;
extern u32 __ctru_linear_heap_size;

void __system_allocateHeaps(void) {
    Handle reslimit = 0;
    if (R_FAILED(svcGetResourceLimit(&reslimit, CUR_PROCESS_HANDLE))) svcBreak(USERBREAK_PANIC);
    s64 maxCommit = 0, currentCommit = 0;
    ResourceLimitType type = RESLIMIT_COMMIT;
    // SG-08: wait until the free amount has settled rather than for a fixed 64 MB. The old test took the first reading
    // over 64 MB, even mid-release, and on a console whose app memory never frees 64 MB (an old 3DS) it waited the full
    // 3 s on every start. Now: 64 MB or more that held for 50 ms, or any amount that held for 300 ms; 3 s at most.
    s64 last = -1;
    int stable = 0;
    for (int tries = 0; tries < 300; tries++) {
        svcGetResourceLimitLimitValues(&maxCommit, reslimit, &type, 1);
        svcGetResourceLimitCurrentValues(&currentCommit, reslimit, &type, 1);
        s64 freeNow = maxCommit - currentCommit;
        stable = (freeNow == last) ? stable + 1 : 0;
        last = freeNow;
        if ((freeNow >= (64ll << 20) && stable >= 5) || stable >= 30) break;
        svcSleepThread(10000000LL);
    }
    svcCloseHandle(reslimit);

    const u32 reserve = 4u << 20;
    u32 remaining = (u32)(maxCommit - currentCommit) & ~0xFFFu;
    remaining = remaining > reserve * 4 ? remaining - reserve : remaining;
    const u32 heap_max = OS_HEAP_AREA_END - OS_HEAP_AREA_BEGIN;
    const u32 linear_want = (32u << 20);
    u32 heap = remaining > linear_want ? remaining - linear_want : remaining / 2;
    if (heap > heap_max) heap = heap_max;
    heap &= ~0xFFFu;
    __ctru_heap_size = heap;
    __ctru_linear_heap_size = remaining - heap;

    if (R_FAILED(svcControlMemory(&__ctru_heap, OS_HEAP_AREA_BEGIN, 0x0, __ctru_heap_size, MEMOP_ALLOC,
                                  (MemPerm)(MEMPERM_READ | MEMPERM_WRITE))))
        svcBreak(USERBREAK_PANIC);
    if (R_FAILED(svcControlMemory(&__ctru_linear_heap, 0x0, 0x0, __ctru_linear_heap_size, MEMOP_ALLOC_LINEAR,
                                  (MemPerm)(MEMPERM_READ | MEMPERM_WRITE))))
        svcBreak(USERBREAK_PANIC);
    mappableInit(OS_MAP_AREA_BEGIN, OS_MAP_AREA_END);
    fake_heap_start = (char*)__ctru_heap;
    fake_heap_end = fake_heap_start + __ctru_heap_size;
}
