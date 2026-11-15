CMD_PCOPY
        ; The native statement boundary owns Y/U across this private command.
        jsr     GFX_PAGE
        bcs     GFX_COPY_ERROR
        tfr     d,u
        jsr     MULTI_COMMA
        jsr     GFX_PAGE
        bcs     GFX_COPY_ERROR
        tfr     d,y
        lbsr     CMD_REQUIRE_END
        jmp     GFX_COPY_PAGE
GFX_COPY_ERROR
        rts
