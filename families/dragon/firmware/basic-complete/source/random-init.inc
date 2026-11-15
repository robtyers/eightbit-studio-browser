; Original initial seed from approved numeric-state observations.
RND_INIT
        ldd     #$804f
        std     $0115
        ldd     #$c752
        std     $0117
        lda     #$59
        sta     $0119
        rts
