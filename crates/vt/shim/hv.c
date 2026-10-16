#include "hv.h"

#include <ghostty/vt.h>
#include <stdlib.h>

struct HvTerm {
  GhosttyTerminal term;
  GhosttyRenderState rs;
  GhosttyRenderStateRowIterator rows;
  GhosttyRenderStateRowCells cells;
  HvWritePty writer;
  void* writer_ud;
};

static uint32_t rgb(GhosttyColorRgb c) {
  return ((uint32_t)c.r << 16) | ((uint32_t)c.g << 8) | (uint32_t)c.b;
}

static void on_write_pty(GhosttyTerminal term, void* ud, const uint8_t* data, size_t len) {
  (void)term;
  HvTerm* t = (HvTerm*)ud;
  if (t->writer) t->writer(t->writer_ud, data, len);
}

HvTerm* hv_new(uint16_t cols, uint16_t rows, size_t max_scrollback) {
  HvTerm* t = (HvTerm*)calloc(1, sizeof(HvTerm));
  if (!t) return NULL;
  if (ghostty_terminal_new(NULL, &t->term, cols, rows) != GHOSTTY_SUCCESS) goto fail;
  if (ghostty_render_state_new(NULL, &t->rs) != GHOSTTY_SUCCESS) goto fail;
  if (ghostty_render_state_row_iterator_new(NULL, &t->rows) != GHOSTTY_SUCCESS) goto fail;
  if (ghostty_render_state_row_cells_new(NULL, &t->cells) != GHOSTTY_SUCCESS) goto fail;
  ghostty_terminal_set(t->term, GHOSTTY_TERMINAL_OPT_USERDATA, t);
  ghostty_terminal_set(t->term, GHOSTTY_TERMINAL_OPT_WRITE_PTY, (const void*)on_write_pty);
  ghostty_terminal_set(t->term, GHOSTTY_TERMINAL_OPT_SCROLLBACK_MAX_LINES, &max_scrollback);
  return t;
fail:
  hv_free(t);
  return NULL;
}

void hv_free(HvTerm* t) {
  if (!t) return;
  if (t->cells) ghostty_render_state_row_cells_free(t->cells);
  if (t->rows) ghostty_render_state_row_iterator_free(t->rows);
  if (t->rs) ghostty_render_state_free(t->rs);
  if (t->term) ghostty_terminal_free(t->term);
  free(t);
}

void hv_set_writer(HvTerm* t, HvWritePty cb, void* userdata) {
  t->writer = cb;
  t->writer_ud = userdata;
}

void hv_write(HvTerm* t, const uint8_t* data, size_t len) {
  ghostty_terminal_vt_write(t->term, data, len);
}

int hv_resize(HvTerm* t, uint16_t cols, uint16_t rows, uint32_t cell_w, uint32_t cell_h) {
  return ghostty_terminal_resize(t->term, cols, rows, cell_w, cell_h) == GHOSTTY_SUCCESS ? 0 : -1;
}

void hv_scroll(HvTerm* t, intptr_t delta) {
  GhosttyTerminalScrollViewport sv = {0};
  if (delta == 0) {
    sv.tag = GHOSTTY_SCROLL_VIEWPORT_BOTTOM;
  } else {
    sv.tag = GHOSTTY_SCROLL_VIEWPORT_DELTA;
    sv.value.delta = delta;
  }
  ghostty_terminal_scroll_viewport(t->term, sv);
}

static uint16_t style_flags(const GhosttyStyle* s) {
  uint16_t f = 0;
  if (s->bold) f |= HV_BOLD;
  if (s->italic) f |= HV_ITALIC;
  if (s->underline) f |= HV_UNDERLINE;
  if (s->strikethrough) f |= HV_STRIKE;
  if (s->faint) f |= HV_FAINT;
  if (s->invisible) f |= HV_INVISIBLE;
  return f;
}

static void read_row(HvTerm* t, HvCell* row, uint16_t cols, uint32_t dfg, uint32_t dbg) {
  uint16_t x = 0;
  while (x < cols && ghostty_render_state_row_cells_next(t->cells)) {
    HvCell* c = &row[x++];
    uint32_t glen = 0;
    ghostty_render_state_row_cells_get(t->cells, GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_LEN, &glen);
    c->cp = 0;
    if (glen > 0) {
      uint32_t buf[16];
      if (glen <= 16) {
        ghostty_render_state_row_cells_get(t->cells, GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_BUF, buf);
        c->cp = buf[0];
      }
    }
    GhosttyColorRgb col;
    c->fg = ghostty_render_state_row_cells_get(t->cells, GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_FG_COLOR, &col) == GHOSTTY_SUCCESS ? rgb(col) : dfg;
    c->bg = ghostty_render_state_row_cells_get(t->cells, GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_BG_COLOR, &col) == GHOSTTY_SUCCESS ? rgb(col) : dbg;
    c->flags = 0;
    bool styled = false;
    ghostty_render_state_row_cells_get(t->cells, GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_HAS_STYLING, &styled);
    if (styled) {
      GhosttyStyle s = GHOSTTY_INIT_SIZED(GhosttyStyle);
      ghostty_render_state_row_cells_get(t->cells, GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_STYLE, &s);
      c->flags = style_flags(&s);
      if (s.inverse) {
        uint32_t tmp = c->fg;
        c->fg = c->bg;
        c->bg = tmp;
      }
    }
  }
  for (; x < cols; x++) row[x] = (HvCell){0, dfg, dbg, 0, 0};
}

int hv_snapshot(HvTerm* t, HvCell* out, size_t cap, uint16_t* cols, uint16_t* rows,
                HvCursor* cursor, uint32_t* default_bg, int force) {
  if (ghostty_render_state_update(t->rs, t->term) != GHOSTTY_SUCCESS) return -1;

  GhosttyRenderStateDirty dirty = GHOSTTY_RENDER_STATE_DIRTY_FALSE;
  ghostty_render_state_get(t->rs, GHOSTTY_RENDER_STATE_DATA_DIRTY, &dirty);
  if (dirty == GHOSTTY_RENDER_STATE_DIRTY_FALSE && !force) return 0;

  uint16_t c = 0, r = 0;
  ghostty_render_state_get(t->rs, GHOSTTY_RENDER_STATE_DATA_COLS, &c);
  ghostty_render_state_get(t->rs, GHOSTTY_RENDER_STATE_DATA_ROWS, &r);
  *cols = c;
  *rows = r;
  if ((size_t)c * r > cap) return -2;

  GhosttyRenderStateColors colors = GHOSTTY_INIT_SIZED(GhosttyRenderStateColors);
  ghostty_render_state_get(t->rs, GHOSTTY_RENDER_STATE_DATA_COLORS, &colors);
  uint32_t dfg = rgb(colors.foreground), dbg = rgb(colors.background);
  *default_bg = dbg;

  GhosttyRenderStateCursor cur = GHOSTTY_INIT_SIZED(GhosttyRenderStateCursor);
  ghostty_render_state_get(t->rs, GHOSTTY_RENDER_STATE_DATA_CURSOR, &cur);
  cursor->visible = cur.visible && cur.viewport_has_value;
  cursor->x = cur.viewport_has_value ? cur.viewport_x : 0;
  cursor->y = cur.viewport_has_value ? cur.viewport_y : 0;
  switch (cur.visual_style) {
    case GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR: cursor->style = 1; break;
    case GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_UNDERLINE: cursor->style = 2; break;
    case GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK_HOLLOW: cursor->style = 3; break;
    default: cursor->style = 0; break;
  }

  ghostty_render_state_get(t->rs, GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR, &t->rows);
  uint16_t y = 0;
  while (y < r && ghostty_render_state_row_iterator_next(t->rows)) {
    ghostty_render_state_row_get(t->rows, GHOSTTY_RENDER_STATE_ROW_DATA_CELLS, &t->cells);
    read_row(t, out + (size_t)y * c, c, dfg, dbg);
    y++;
  }
  for (; y < r; y++)
    for (uint16_t x = 0; x < c; x++) out[(size_t)y * c + x] = (HvCell){0, dfg, dbg, 0, 0};

  ghostty_render_state_clean(t->rs);
  return 1;
}
