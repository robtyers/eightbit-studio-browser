; Original rectangle perimeter using the shared guest line kernel.
GFX_BOX
        ldd     -8,u
        std     12,u
        ldd     -6,u
        std     14,u
        ldd     -4,u
        pshs    d
        ldd     -2,u
        pshs    d
        ldd     14,u
        std     -2,u
        bsr     NATIVE_ISLAND_GRAPHICS_BOX_INC_0
        ldd     ,s
        std     -2,u
        bsr     NATIVE_ISLAND_GRAPHICS_BOX_INC_0
        ldd     12,u
        std     -4,u
        bsr     NATIVE_ISLAND_GRAPHICS_BOX_INC_0
        ldd     14,u
        std     -2,u
        bsr     NATIVE_ISLAND_GRAPHICS_BOX_INC_0
        puls    d
        std     -2,u
        puls    d
        std     -4,u
        rts
NATIVE_ISLAND_GRAPHICS_BOX_INC_0
        jmp     GFX_LINE

