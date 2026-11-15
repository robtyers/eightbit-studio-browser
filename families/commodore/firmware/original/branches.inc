; Original fixed conditional-jump expansion; no external macro library.
.macro jeq target
    bne *+5
    jmp target
.endmacro
.macro jne target
    beq *+5
    jmp target
.endmacro
.macro jcc target
    bcs *+5
    jmp target
.endmacro
.macro jcs target
    bcc *+5
    jmp target
.endmacro
