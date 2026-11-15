; Original ROM-parameterised equality-chain template. Apache-2.0.
; Inline data: nonzero count, then count pairs (byte key, absolute target word).
; On equality, resume the named target. Otherwise resume after all pairs.
; D/X/Y/U/DP and S retain the original chain values. NZVC are exactly the last
; CMPA (the matching key or last failed key); E/F/H/I are unaffected. Four
; transient bytes save D/X, and the JSR continuation becomes the selected PC.
BYTE_EQUAL_DISPATCH
        pshs    d,x
        ldx     4,s
        ldb     ,x+
BYTE_EQUAL_NEXT
        cmpa    ,x+
        beq     BYTE_EQUAL_FOUND
        leax    2,x
        decb
        bne     BYTE_EQUAL_NEXT
        stx     4,s
        cmpa    -3,x
        puls    d,x,pc
BYTE_EQUAL_FOUND
        ldx     ,x
        stx     4,s
        cmpa    ,s
        puls    d,x,pc
