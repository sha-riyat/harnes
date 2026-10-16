/* Flat C API over libghostty-vt, easy to bind from Rust. */
#pragma once
#include <stddef.h>
#include <stdint.h>

typedef struct HvTerm HvTerm;

/* One rendered cell. Colours are 0x00RRGGBB. */
typedef struct {
  uint32_t cp;    /* first codepoint, 0 = empty */
  uint32_t fg;
  uint32_t bg;
  uint16_t flags; /* HV_* bits */
  uint16_t _pad;
} HvCell;

enum {
  HV_BOLD = 1 << 0,
  HV_ITALIC = 1 << 1,
  HV_UNDERLINE = 1 << 2,
  HV_STRIKE = 1 << 3,
  HV_FAINT = 1 << 4,
  HV_INVISIBLE = 1 << 5,
};

typedef struct {
  uint16_t x, y;
  uint8_t visible; /* 0/1 */
  uint8_t style;   /* 0 block, 1 bar, 2 underline, 3 hollow */
} HvCursor;

/* Called with bytes the terminal wants written back to the PTY
 * (query replies, mode reports). */
typedef void (*HvWritePty)(void* userdata, const uint8_t* data, size_t len);

HvTerm* hv_new(uint16_t cols, uint16_t rows, size_t max_scrollback);
void hv_free(HvTerm* t);
void hv_set_writer(HvTerm* t, HvWritePty cb, void* userdata);
void hv_write(HvTerm* t, const uint8_t* data, size_t len);
int hv_resize(HvTerm* t, uint16_t cols, uint16_t rows, uint32_t cell_w, uint32_t cell_h);
/* delta < 0 scrolls up into history; 0 jumps back to the bottom. */
void hv_scroll(HvTerm* t, intptr_t delta);

/* Fills `out` with cols*rows cells (row-major) when the frame is dirty.
 * Returns 1 if a frame was written, 0 if clean, <0 on error.
 * `force` writes the frame even when clean. */
int hv_snapshot(HvTerm* t, HvCell* out, size_t cap, uint16_t* cols, uint16_t* rows,
                HvCursor* cursor, uint32_t* default_bg, int force);
