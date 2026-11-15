; Original byte alignment and checked block input. Public $81/Z report status.
TAPE_BLOCK_IN
        pshs    d,x,y,u
        clr     $81
        ldu     #4096
        clra
        pshs    a
TAPE_FIND_SYNC
        jsr     TAPE_BIT_IN
        ror     ,s
        tst     $81
        bne     TAPE_INPUT_ERROR
        lda     ,s
        cmpa    #$3c
        beq     TAPE_INPUT_HEADER
        leau    -1,u
        cmpu    #0
        bne     TAPE_FIND_SYNC
        lda     #3
        sta     $81
        bra     TAPE_INPUT_ERROR
TAPE_INPUT_HEADER
        leas    1,s
        jsr     TAPE_BYTE_IN
        bcs     TAPE_INPUT_DONE
        sta     $7c
        sta     $80
        jsr     TAPE_BYTE_IN
        bcs     TAPE_INPUT_DONE
        sta     $7d
        adda    $80
        sta     $80
        clra
        ldb     $7d
        addd    $7e
        bcs     TAPE_INPUT_BOUNDS
        cmpd    #$8000
        bhi     TAPE_INPUT_BOUNDS
        ldx     $7e
        ldb     $7d
        beq     TAPE_INPUT_CHECKSUM
TAPE_INPUT_DATA
        jsr     TAPE_BYTE_IN
        bcs     TAPE_INPUT_DONE
        sta     ,x+
        adda    $80
        sta     $80
        decb
        bne     TAPE_INPUT_DATA
TAPE_INPUT_CHECKSUM
        jsr     TAPE_BYTE_IN
        bcs     TAPE_INPUT_DONE
        cmpa    $80
        beq     TAPE_INPUT_DONE
        lda     #1
        sta     $81
TAPE_INPUT_DONE
        tst     $81
        beq     TAPE_INPUT_OK
        orcc    #1
        puls    d,x,y,u,pc
TAPE_INPUT_OK
        andcc   #$fe
        puls    d,x,y,u,pc
TAPE_INPUT_BOUNDS
        lda     #2
        sta     $81
        bra     TAPE_INPUT_DONE
TAPE_INPUT_ERROR
        leas    1,s
        bra     TAPE_INPUT_DONE

