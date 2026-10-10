/* Original Eightbit Studio adapter, GPL-3.0-or-later.
 * Uses the unmodified zlib-licensed floooh/chips C64 PAL API. No firmware embedded. */
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#define CHIPS_IMPL
#include "chips/chips_common.h"
#include "chips/m6502.h"
#include "chips/m6526.h"
#include "chips/m6569.h"
#include "chips/m6581.h"
#include "chips/kbd.h"
#include "chips/mem.h"
#include "chips/clk.h"
#include "systems/c1530.h"
#include "chips/m6522.h"
#include "systems/c1541.h"
#include "systems/c64.h"
static c64_t computer;
static uint32_t pixels[M6569_FRAMEBUFFER_SIZE_BYTES];
static float samples[8192];
static unsigned sample_count;
static bool ready;
static void audio(const float *data, int count, void *unused) {
    (void)unused;
    if (count > 0 && (unsigned)count <= 8192 - sample_count) {
        memcpy(samples + sample_count, data, (unsigned)count * sizeof(float));
        sample_count += (unsigned)count;
    }
}
int studio_init(uint8_t *roms, unsigned length) {
    if (!roms || length != 20480) return 0;
    c64_desc_t config = {0};
    config.roms.basic = (chips_range_t){roms,8192};
    config.roms.kernal = (chips_range_t){roms+8192,8192};
    config.roms.chars = (chips_range_t){roms+16384,4096};
    config.audio.callback.func = audio;
    config.audio.sample_rate = 44100;
    config.audio.num_samples = 512;
    c64_init(&computer, &config);
    ready = true;
    c64_exec(&computer, 2500000);
    sample_count = 0;
    return 1;
}
int studio_load(uint8_t *media, unsigned length) {
    if (!ready || !media || length < 9 || length > 38913 || media[0] != 1 || media[1] != 8) return 0;
    return c64_quickload(&computer, (chips_range_t){media,length}) ? 1 : 0;
}
void studio_step(unsigned micros) { if (ready && micros <= 50000) c64_exec(&computer, micros); }
void studio_key(unsigned code, unsigned down) {
    if (!ready || code > 255) return;
    if (down) c64_key_down(&computer, (int)code); else c64_key_up(&computer, (int)code);
}
uint32_t *studio_pixels(void) {
    chips_display_info_t info = c64_display_info(&computer);
    const uint8_t *indices = info.frame.buffer.ptr;
    const uint32_t *palette = info.palette.ptr;
    for (unsigned i=0;i<M6569_FRAMEBUFFER_SIZE_BYTES;i++) pixels[i] = indices[i] < info.palette.size/4 ? palette[indices[i]] : 0xff000000;
    return pixels;
}
int studio_width(void) { return M6569_FRAMEBUFFER_WIDTH; }
int studio_height(void) { return M6569_FRAMEBUFFER_HEIGHT; }
float *studio_audio(void) { return samples; }
unsigned studio_audio_count(void) { return sample_count; }
void studio_audio_clear(void) { sample_count = 0; }
void studio_stop(void) {
    if (ready) c64_discard(&computer);
    memset(&computer,0,sizeof computer); memset(samples,0,sizeof samples); sample_count=0; ready=false;
}
