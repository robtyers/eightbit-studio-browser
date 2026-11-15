; Original LIST/LLIST/DEL range parser. U addresses two inclusive words.
; No programme or public pointer is changed until the whole range is valid.
LINE_RANGE
        clra
        clrb
        std     ,u
        ldd     #63999
        std     2,u
        jsr     EX_SPACE
        cmpa    #$c4
        beq     RANGE_UPPER
        jsr     EX_END_CHAR
        beq     RANGE_END
        jsr     PARSE_LINE_NUMBER
        bcs     RANGE_ERROR
        std     ,u
        std     2,u
        jsr     EX_SPACE
        cmpa    #$c4
        bne     RANGE_END
        ldd     #63999
        std     2,u
RANGE_UPPER
        leax    1,x
        jsr     EX_END
        beq     RANGE_END
        jsr     PARSE_LINE_NUMBER
        bcs     RANGE_ERROR
        std     2,u
RANGE_END
        jsr     CMD_REQUIRE_END
        ldd     2,u
        cmpd    ,u
        blo     RANGE_ERROR
        rts


RANGE_ERROR
        coma
        rts
