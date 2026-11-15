INPUT_RESTORE_TYPE
        ; A bad numeric reply may have cleared its expression type. Recover it
        ; from the destination name without allocating another variable record.
        lda     7,u
        sta     $06
        lbra    INPUT_PROMPT
INPUT_SUCCESS
        clra
INPUT_RETURN
        leas    264,s
        rts

