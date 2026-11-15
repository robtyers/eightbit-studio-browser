; Original unsigned integer conversion. Both paths share the same native
; normaliser; retaining D's high bit avoids floating addition and cursor saves.
UNSIGNED_TO_FAC
        clr     $06
        clr     FSIGN
        jmp     INT_POSITIVE

