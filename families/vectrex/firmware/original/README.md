# Original Vectrex development executive

Original Apache-2.0 guest 6809 source. No historical executive, Mine Storm,
commercial cartridge, emulator source or recovered disassembly is included.
Hardware and interface facts come from the manufacturer's Programmer's Manual
[volume I](https://www.vectrex.de/wp-content/uploads/2013/12/vecprogman_v1.pdf)
and [volume II](https://www.vectrex.de/wp-content/uploads/2013/12/vecprogman_v2.pdf).
The address/name index in docs/vectrex-original-specification.md is supplementary.
Neither manual listings nor third-party implementation code are implementation
inputs. The font's 80 glyphs were independently drawn on a 5 × 7 grid here.

The portable runtime assembles executive.asm with the original no-I/O 6809
assembler. It occupies the stock 8 KiB $E000–$FFFF ROM window and consumes no
cartridge space, private pages or reserved RAM. Temporary working state uses the
caller's S stack. manifest.json binds source and assembled image hashes:

```sh
cargo run --quiet --offline -p eightbit-vectrex-emulator --example identity
```

## Development ABI 0.5

The executive boots the conventional cartridge header as guest code. The following
entries are implemented; DP must be $D0 except for boot, DP setup and Wait_Recal.
Y and U are preserved unless stated; every returning call balances S. A/B and
condition flags may change unless a preservation guarantee is listed. This is an
explicit development contract, not complete historical ABI qualification.

| Entry | Service | Verified outputs and RAM effects |
| --- | --- | --- |
| $F000 | Cold boot | Clear RAM, initialise VIA, default text size $F850 and refresh $3075, enter cartridge. |
| $F192 | Wait_Recal | Reload T2 from $C83D/E (low/high); increment big-endian $C825/6; zero beam; D=$0301, X=$F9F4, DP=$D0. |
| $F1AA / $F1AF | DP_to_D0 / DP_to_C8 | A and DP become $D0 / $C8; preserve B/X/Y/U. |
| $F1BA | Read_Btns | Current/previous state $C80F/10; new-press mask $C811 and individual masked bits $C812–19; A=mask, B=0, X=$C81A. |
| $F1F8 | Joy_Digital | Enabled axes $C81F–22 select $C81B–1E; signed -1/0/+1; preserve D/X/Y/U. Disabled axes return zero. |
| $F256 | Sound_Byte | A=register 0–13, B=data; write PSG and $C800+A mirror; X=$C800, B=1. |
| $F29D / $F2A1 / $F2A5 / $F2A9 / $F2AB | Intensity | Fixed $1F/$3F/$5F/$7F or A masked to 7 bits; mirror $C827; D=$0501. |
| $F312 | Moveto_d | Relative signed (A=Y,B=X), current T1 scale; preserve D/X. |
| $F354 | Reset0Ref | Zero beam/reference; D=$0301, preserve X. |
| $F3DF | Draw_Line_d | First relative pair in D; advance X by two; use LIST/$C823 for any subsequent pairs; honour ZSKIP/$C824 on completion. Set LIST=0. |

The 0.1 executive did not implement all these documented register/RAM effects.
In particular, Read_Btns now returns the **new-press** mask rather than held state;
read $C80F for held buttons. Wait_Recal increments the full two-byte counter.
Intensity and sound now maintain their public mirrors. Draw_Line_d advances X.
Cartridges relying on the earlier development side effects need adjustment.
The schema-1 project, starter bytes, external routes and ordinary Run are unchanged.

## Counted vector lists

| Entry | Draw every pair | Move to first pair, then draw |
| --- | --- | --- |
| Count prefix at X | $F3CE | $F3AD |
| Count then scale prefixes at X | $F3D6 | $F3B5 |
| Count in $C823, scale in B | $F3D2 | $F3B1 |
| Count in A, scale in B | $F3D8 | $F3B7 |
| Count in A, current scale | $F3DA | $F3B9 |
| Count in $C823, current scale | $F3DD | $F3BC |
| First pair in D, remaining pairs at X+2 | $F3DF | $F3BE |

Count means pairs minus one: all byte values 0–255 are finite (1–256 pairs).
Each pair is signed Y,X. X returns immediately after the final pair; LIST returns
zero. A non-zero ZSKIP zeros the beam/reference after the entire list; zero leaves
it at the endpoint. D is scratch. Y/U/DP are preserved. A one-pair move-only list
emits no visible stroke. Patterned lists have their separate contract below; grid formats remain unavailable.

## Packet vector lists

| Entry | Scale source |
| --- | --- |
| $F404 (PACK2X) | $FF |
| $F408 (PACK1X) | $7F |
| $F40C (LPACK) | First byte at X, consumed before the list |
| $F40E (TPACK) | B |
| $F410 (PACKET) | Current T1 scale |

Each vector is a mode,Y,X triple: $00 moves blanked, $FF draws a solid stroke,
and a single $01 ends the list. Y/X are signed bytes. Zero to 256 vectors are
accepted, followed by the terminator. Other modes, a 257th vector, a missing
terminator or a read that would wrap the 16-bit address space fault with VXO1001.
This original contract returns X **on the terminator**, without consuming it;
the manual describes this as the end of the list. Exact historical return-pointer
qualification is separate. The selected scale remains set. Non-zero ZSKIP/$C824
zeros the beam after completion; zero leaves the endpoint. D is scratch; Y/U/DP,
LIST/$C823 and other public RAM are preserved. Only S is temporary storage.
Cancellation is checked at guest instruction boundaries, including during motion.
The portable session releases controls on fault and supports Reset/Stop recovery.
These packet modes remain solid/blank only; dashed packets use the separate $F46E contract below. Grid lists and rotated packets remain unavailable.

## Patterned vector lists

Profile 0.5 implements DSHDF ($F434: A = pairs minus one), DASHDF ($F437:
LIST/$C823 = pairs minus one) and DASHPK ($F46E). DASH/$C829 is an eight-bit
pattern read without modification. Counted lists contain 1–256 signed Y,X pairs;
return X after the final pair and LIST=0. Reject a pair whose advancing pointer
would wrap. The count is not a terminator, so all byte values are finite.

DASHPK reads mode,Y,X triples: $00 blank, $02 solid, $FF patterned, and a single
$01 terminator. Unlike PACKET ($F410), $FF means patterned here. Require LIST=0
on entry; it remains zero. Accept 0–256 vectors and return X **after the terminator**.
Invalid modes, a 257th vector, missing terminator, non-zero LIST or a read/advance
that would wrap produces VXO1001. Earlier valid strokes may already have drawn.

All three use current T1 scale and honour ZSKIP/$C824 consistently: non-zero
zeros the beam after the entire list; zero leaves its endpoint. D is scratch;
Y/U/DP, DASH, T1 scale, T2 configuration and incoming ACR are preserved. Return
with PCR=$CE (beam blanked). S holds temporary state; only LIST may change in
public RAM. Cancellation remains available during validation and motion;
faults release controls and Reset/Stop recover through the portable session.

The original guest sets shift mode 4 during each patterned stroke, loads DASH
anew, uses CB2 output and restores ACR after blanking. In the existing board model,
bit 7 starts visible immediately after the load; the first shift tick repeats it,
then bits rotate most-significant first every T2-low-latch + 2 CPU cycles. A vector
starts after guest setup, so its phase includes that setup time. No T2 latch is
written and there is no hidden pattern RAM or host drawing routine. $00 is blank,
$FF solid; mixed patterns expose gaps at suitable scale and bit period. These are
explicit original digital semantics; historical dash phase, exact timing, VIA
postconditions, analogue/CRT behaviour and physical acceptance remain unqualified.
The manual flags inconsistent historical zeroing; this original contract always
honours ZSKIP. DSHDF1 ($F433) and $F439 remain unsupported; no alias or zero-count
semantics are assumed.

## Text

$F373 (Print_Str_hwyx) reads height,width,Y,X at U; $F378 (Print_Str_yx) reads Y,X;
$F37A (Print_Str_d) takes Y,X in D. These zero and position with scale $7F.
$F495 (Print_Str) starts at the current beam position. All consume text at U through
an exact $80 terminator, return U after it, D=$0301, X=$FBB4, preserve Y/DP and
zero the beam on return. $C82C/D records the text start; $F373 also updates the
height/width at $C82A/B. The incoming scale is restored for $F495; positioned
entries leave $7F. No historical character table exists at the returned X address.

Supported characters are $20–$6F, with at most 256 characters per call. Other
bytes or a missing terminator fail with VXO1001 through an unsupported guest
instruction. An empty string is valid. Cancellation, Stop and Reset remain bounded
by the portable session, including while a long string is executing.

The independently designed geometry is five columns, seven rows and one blank
column, drawn by guest VIA motions. Each cell uses a 16-cycle timer; signed
width divided by eight (arithmetic shift) sets its X velocity, and signed height
sets its row Y velocity. This is an idealised original font, not the historic
raster font, exact spacing or timing. Rendering may straddle the board's fixed
display frames, so a single capture can contain only part of a scene; calibrated
phosphor persistence is absent.
Only stack and the published text pointer/size bytes are working storage.

## Text lists

$F385 (TXTSIZ) reads repeated height,width,Y,X,text,$80 records from U.
$F38C (TXTPOS) reads repeated Y,X,text,$80 records using $C82A/B for size.
A single zero at the start of the next record ends either list: zero height for
TXTSIZ, zero Y for TXTPOS. Use the single-string services when that first field
needs to be zero. TXTSIZ may position at Y=0 because height is its first field.

The original development limit is 64 messages, each with up to 256 characters
in $20–$6F followed by $80. Empty messages and empty lists are valid. Return
D=$0001, X=$FBB4 and U immediately **after the final list zero**; preserve Y/DP
and balance S. $C82C/D holds the final message's text pointer. TXTSIZ leaves the
final height/width at $C82A/B; TXTPOS preserves those size bytes. Non-empty lists
leave scale $7F and a zeroed beam. Empty lists leave size, message pointer, scale
and beam unchanged. No historical character table is installed at X.

Each header and message is scanned in guest code before that message is drawn.
A 65th message, unsupported character, missing terminator, overlong string or
16-bit pointer wrapping produces VXO1001. Earlier valid messages may already
have drawn when a later record fails. The outer count and format are private S
stack values; only published text size/pointer RAM changes. Cancellation checks
remain at instruction boundaries during validation and rendering; session faults
release controls and Reset/Stop recover. $F38A stays unsupported: it has no entry
contract in these manufacturer references. No alias behaviour is inferred.

## Qualification and exclusions

`cargo test --offline -p eightbit-vectrex-emulator` checks the existing starter,
register/RAM contracts, pointer/stack boundaries, all 80 glyphs, list counts,
explicit guest faults at 96 unsupported public entries,
equivalence to independently authored individual line/move sequences, fault
recovery and cancellation. `templates/executive-demo.asm` is an optional original
text/list cartridge; `templates/packet-demo.asm` exercises all five packet calls.
`templates/text-list-demo.asm` exercises both text-list formats;
`templates/pattern-demo.asm` exercises the three patterned-list services. All examples
are exercised by the native/WASM parity check; none replaces
the default starter. See docs/testing.md for dated evidence.

Tests check masked flag preservation for the DP/intensity paths; they do not assert
historical final NZVC values for every service. Instruction timing, historical
stack-depth limits and exact VIA postconditions remain unqualified. Frame pacing
is nominally 50 Hz and vectors are idealised. Unlisted entries and interrupt
vectors contain unsupported opcodes and fail explicitly. Executive interrupt
callbacks, unlisted text entry points, unlisted patterned entries, grid lists, music, score/math services,
historical ROM tables and broad legacy games remain outside this subset.
