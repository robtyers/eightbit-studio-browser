GFX_COPY_PAGE
        pshs    x
        ldx     #1536
GFX_COPY_BYTE
        lda     ,u+
        sta     ,y+
        leax    -1,x
        bne     GFX_COPY_BYTE
        puls    x,pc
