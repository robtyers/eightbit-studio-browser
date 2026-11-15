CMD_PCOPY
        pshs    y,u
        jsr     GFX_PAGE
        bcs     GFX_COPY_ERROR
        tfr     d,u
        jsr     MULTI_COMMA
        jsr     GFX_PAGE
        bcs     GFX_COPY_ERROR
        tfr     d,y
        bsr     CMD_REQUIRE_END
        jmp     GFX_COPY_PAGE
GFX_COPY_ERROR
        puls    y,u,pc

