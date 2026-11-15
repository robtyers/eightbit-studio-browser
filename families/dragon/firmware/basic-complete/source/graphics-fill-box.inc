; Original bounded horizontal-span rectangle fill.
GFX_BOX_FILL
        ldd     -8,u
        std     12,u
        ldd     -6,u
        std     14,u
        ldd     -2,u
        pshs    d
BOX_FILL_ROW
        ldd     12,u
        std     -8,u
        ldd     14,u
        std     -6,u
        std     -2,u
        lbsr     NATIVE_ISLAND_GRAPHICS_BOX_INC_0
        ldd     14,u
        cmpd    ,s
        beq     BOX_FILLED
        bhi     BOX_FILL_UP
        addd    #1
        bra     BOX_FILL_NEXT
BOX_FILL_UP
        subd    #1
BOX_FILL_NEXT
        std     14,u
        bra     BOX_FILL_ROW
BOX_FILLED
        puls    d
        std     -2,u
        rts
