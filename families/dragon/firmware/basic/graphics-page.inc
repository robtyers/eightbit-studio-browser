; One BASIC graphics page is 1536 bytes. Both ends must be reserved RAM.
GFX_PAGE
        jsr     GFX_BYTE
        decb
        cmpb    #7
        bhi     GFX_PAGE_ERROR
        bsr     GFX_PAGE_ADDRESS
        addd    #1536
        cmpd    TXTTAB
        bhs     GFX_PAGE_ERROR
        subd    #1536
        rts
GFX_PAGE_ERROR
        coma
        rts
GFX_PAGE_ADDRESS
        lda     #6
        mul
        tfr     b,a
        clrb
        adda    $bc
        rts

