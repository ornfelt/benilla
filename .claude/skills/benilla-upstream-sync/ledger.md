# benilla upstream sync ledger

Maintained by the `benilla-upstream-sync` skill. A hint, not the truth: `git rev-list --reverse
<branch>..main` lists what a branch still lacks.

Source: `main` (origin `ornfelt/benilla`, the fork: upstream benilla's commits, the fork's own
commits and its merges; not linear). Upstream: `samwhosung/benilla` (remote `upstream`).
Fork point of both branches: `ba7fe6e2` release 0.2.0 (#468) (2026-09-27)
gfx-dll-backend merged through: `65a5e44d` Unit aura bindings resolve their token as 1.12.1 does (#497) - 360 remaining
trimmed-bevy merged through: `65a5e44d` Unit aura bindings resolve their token as 1.12.1 does (#497) - 360 remaining
`main` when last fetched: `daf1bc9b` Merge branch 'samwhosung:main' into main (2026-10-23)

Statuses (per branch): `pending` (not looked at yet), `as-is` (merged, the change unchanged,
nothing else needed), `adapted` (merged, and reshaped for the branch or followed by a gfx-side
change - the notes say what), `already-in` (the branch had the change already; the merge kept the
branch's version), `not-applicable` (merged with `-s ours`, nothing of it fits the branch - the
notes say why), `skipped` (a merge that changes nothing, or the maintainer decided not to land it;
a no-change merge recorded by an empty merge carries that merge's sha), `asked` (waiting on the
maintainer, nothing merged on that branch). A branch is level with `main` - nothing behind on
GitHub - when `git rev-list --count <branch>..main` is 0.

Origin: `upstream` (in samwhosung/benilla), `fork` (made on ornfelt/benilla, not in upstream),
`merge` (a merge commit on `main`).

Handling: `as-is` (taken as `main` has it on both branches, apart from the branch rules every
commit gets), `needed` (changed because the branch needed it), `requested` (changed, skipped or
redone because the maintainer asked). Anything but `as-is` has an entry under Special handling.

| # | Commit | Date | Origin | Subject | gfx-dll-backend | trimmed-bevy | Handling | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | `05a9be68` | 2026-09-27 | upstream | UnitBuff and UnitDebuff answer for party1-4 and partypet1-4: the live unit's slots, else the roster record's slot-indexed aura block, with UNIT_AURA on each change (#470) | as-is 892791fe | as-is f5dd909f | as-is | no gfx side: UI script, party records and the group stats parser (wire: the party member stats aura block is now parsed); schedule unchanged (Windows frame dump identical); check with and without gfx and protocol tests green, app lib tests show only the known Windows-only failures; Windows only |
| 2 | `03b10722` | 2026-09-27 | upstream | A script's selection, cast and targeting-cursor calls apply in the order it made them, so a macro's /target then /cast casts at the new target (#473) | as-is 1786c152 | as-is 89d91747 | as-is | no gfx side (script, target and cast logic); trimmed-bevy keeps `mod shaders` deleted in lib.rs and re-records frame.txt's Update section (only this commit's systems and orders change; Update sorts byte-identically on stock and trimmed Bevy); check with and without gfx, app and ui tests show only the known Windows-only failures; Windows only |
| 3 | `bd074c3f` | 2026-09-27 | upstream | After thirty idle minutes the logout warning prints once per logout: the idle leg, line and request, is skipped while a logout is pending (0x482ef5) (#474) | as-is 97a36f0a | as-is 660f39dd | as-is | no gfx side (chat idle and logout logic); trimmed-bevy re-records frame.txt's Update section (only the two new idle_handler orders; Update sorts byte-identically on stock and trimmed Bevy); check with and without gfx, app lib tests show only the known Windows-only failures; Windows only |
| 4 | `0237dbd1` | 2026-09-27 | upstream | Geosets above 1700 stay visible, as 1.12.1 switches off only 0 to 1700: an asset pack's Night Elf eye glow on 1702 shows (#438) | as-is 66ab54c0 | as-is 58dce033 | as-is | no gfx side (geoset selection in benilla-formats; benilla-gfx draws the meshes it is given); schedule unchanged; check with and without gfx, formats tests green, app lib tests show only the known Windows-only failures; Windows only |
| 5 | `655854c1` | 2026-09-27 | upstream | The zoom limit is cameraDistanceMax × cameraDistanceMaxFactor held to [0, 50] by the reference's ordered compares, cameraDistanceMax a registered CVar, and a NaN factor zooms without panicking (#475) | as-is 5577d25e | as-is 2d4e7b5b | as-is | no gfx side (camera zoom and the cameraDistanceMax CVar; the wheel's scroll input is unchanged); schedule unchanged; check with and without gfx, app lib tests show only the known Windows-only failures; Windows only |
| 6 | `3ed9c0b1` | 2026-09-28 | upstream | fix: show the repair cost on item tooltips in repair mode (#469) | as-is 96dff437 | as-is 1324444a | as-is | no gfx side (merchant feed and tooltip script); trimmed-bevy re-records frame.txt's Update section (feed_repair_all_cost renamed feed_repair_costs; Update sorts byte-identically on stock and trimmed Bevy); check with and without gfx, app lib and ui tests show only the known Windows-only failures; Windows only |
| 7 | `208ab10c` | 2026-09-28 | upstream | EnableAllAddOns and DisableAllAddOns switch the Lua index array only, so a disable-all leaves the server-hidden Blizzard_* addons loadable (#480) | as-is 1bea8c4d | as-is ec3223b4 | as-is | no gfx side (benilla-ui addon script); schedule untouched; check with and without gfx, ui tests show only the known Windows-only failure; Windows only |
| 8 | `7362a87f` | 2026-09-28 | upstream | player-tests gets the addon corpus link as $BENILLA_ADDON_CORPUS and BENILLA_REQUIRE_DATA where the corpus is, so its 30 corpus tests run and a skip fails (#482) | as-is b341ff66 | as-is 3cd58e09 | as-is | no gfx side (scripts/gates.sh only); bash -n clean, the gate not run (Windows, no addon corpus) |
| 9 | `06bc451b` | 2026-09-28 | upstream | The 48 movement, camera and targeting functions Bindings.xml calls are Lua globals on their keys' paths: Start/Stop hold the key's command inside a binding body, targeting and AttackTarget go through the call queue (#485) | as-is 7063167a | as-is c4c463f5 | as-is | no gfx side (bindings and script verbs; the new focus-loss clear reads KeyboardFocusLost, which benilla-gfx's window.rs writes as bevy_winit does); schedule unchanged; check with and without gfx, app lib, cover_input_wall and ui tests show only the known Windows-only failures; Windows only |
| 10 | `78862ff2` | 2026-09-28 | upstream | UnitBuff and UnitDebuff answer for raid1-40 and raidpet1-40, each unit's list built once and shared by every token naming it (#489) | as-is b28964e9 | as-is 44138bff | as-is | no gfx side (aura feed and party records); schedule unchanged; check with and without gfx, app lib tests show only the known Windows-only failures; Windows only |
| 11 | `36b359f0` | 2026-09-28 | upstream | Logout and the AddOns screen's Okay write only a changed character's own enable rows, so a character with no row keeps following the realm's others (#418) (#490) | as-is 3d8013fd | as-is 4dd6f563 | as-is | no gfx side (addon enable rows, char-select AddOns screen, UI script); schedule unchanged (Windows frame dump identical before and after on trimmed-bevy); check with and without gfx, ui and app lib tests show only the known Windows-only failures; Windows only |
| 12 | `a6cbb932` | 2026-09-28 | upstream | Guid-tail channel notices are delivered through the channel stamp, so a member's join or leave, a kick, a ban and an owner or moderation change print numbered in the stock chat frame (#491) | as-is e94bdf61 | as-is 5da880dd | as-is | no gfx side (chat channel notice feed); schedule unchanged (Windows frame dump identical before and after on trimmed-bevy); check with and without gfx, app lib tests show only the known Windows-only failures; Windows only |
| 13 | `476da9b2` | 2026-09-28 | upstream | GetAuctionItemInfo names the bidder on the player's own owner-list auctions, nil until the name lands, and 1 or nil on every other row (#259) (#492) | as-is cbc08655 | as-is 0fdcec39 | as-is | no gfx side (auction owner-list bidder name, auction script); schedule unchanged (Windows frame dump identical before and after on trimmed-bevy); check with and without gfx, app lib, auction_frame and ui tests show only the known Windows-only failures; Windows only |
| 14 | `8d31e9e9` | 2026-09-28 | upstream | A crate on top of benilla adds its own Bevy plugins through `benilla_app::run_with`, and a client started through it reads `extended` in its build line (#495) | as-is 8535b13f | adapted fdd5bc5f | needed | no gfx side (launcher entry point, build id, crash report); trimmed-bevy keeps the ExtractSchedule flip deleted and adapts the example's documented manifest to its 52 root patches so launcher_shape holds (see Special handling); schedule unchanged (Windows frame dump identical before and after on trimmed-bevy); check with and without gfx (the example builds on both), app lib, launcher_shape and benilla-world tests show only the known Windows-only failures; Windows only |
| 15 | `65a5e44d` | 2026-09-28 | upstream | Unit aura bindings resolve their token as 1.12.1 does, so mouseover, party1target and any casing read the unit they name (#497) | as-is d926fc04 | as-is f6105773 | as-is | no gfx side (unit aura token resolution in benilla-ui's new unit/resolve.rs, the aura feed's group.rs renamed units.rs, UI script); feature-off diff unchanged; schedule unchanged (the Windows frame dump differs from frame.txt only by the Linux-only layout_keys::remap); check with and without gfx, app lib and ui tests show only the known Windows-only failures; Windows only |
| 16 | `c6767d6a` | 2026-09-28 | upstream | SendChatMessage resolves its language name against Languages.dbc, raising "Unknown language" on a miss, and the chat send carries that id, so a racial language picked in the chat menu is spoken (#164) (#498) | pending | pending | - |  |
| 17 | `bacbb791` | 2026-09-28 | upstream | The player's Attack runs the attack validator's target pick: a friend selected retargets the nearest enemy in call order, a corpse or a fruitless scan says "You cannot attack that target.", nothing selected and no enemy says "There is nothing to attack." (#249) (#501) | pending | pending | - |  |
| 18 | `1a37909c` | 2026-09-28 | upstream | The mouseover unit carries the hovered guid, so UnitIsUnit("mouseover", "target") answers 1 over the target: unit snapshots take their guid (#502) | pending | pending | - |  |
| 19 | `6633a11d` | 2026-09-28 | upstream | A dev build of a crate on top of benilla finds its install, `benilla-config/` and `.probe-identity` in its own folder, which its build stamp hands in, not in cargo's checkout of benilla (#504) | pending | pending | - |  |
| 20 | `829194ac` | 2026-09-28 | upstream | SendChatMessage sends nothing for an empty line unless its type is AFK or DND, checked after the chat type as 1.12.1 does (#499) (#505) | pending | pending | - |  |
| 21 | `0e39273f` | 2026-09-28 | upstream | "mouseover" names nobody once no unit wins the hover: empty ground, a corpse or a nearer GameObject clear it, and the fading tooltip's health bar follows the unit's guid, not the token (#518) | pending | pending | - |  |
| 22 | `ae6d9f35` | 2026-09-28 | upstream | MouselookStart and MouselookStop start and end mouse look with no button held, and a look session outlives losing focus with the OS cursor handed back (#534) | pending | pending | - |  |
| 23 | `db94ea03` | 2026-09-28 | upstream | fix: show spell tooltips whole on the first hover (#472) | pending | pending | - |  |
| 24 | `44fa1813` | 2026-09-28 | upstream | SetCVar and RegisterCVar read their arguments as 1.12.1 does: a number as %.14g text, "0" for a value with no string form, the reference usage text, realmList/realmName/scriptMemory read-only in the world, and CVAR_UPDATE fired inside the call for every token (#557) | pending | pending | - |  |
| 25 | `22634279` | 2026-09-28 | upstream | benilla's own interface loads as one layer after the stock FrameXML and before the addons; the core loads StatsFrame.xml and runs LocaleProperties.lua once, as FrameXML.toc does (#562) | pending | pending | - |  |
| 26 | `6e869eea` | 2026-09-28 | upstream | The core loads the stock GameMenuFrame.xml, and the layer reshapes it in place into benilla's menu, keeping every stock rung in the chain anchored TOP (#564) | pending | pending | - |  |
| 27 | `a21fb275` | 2026-09-28 | upstream | The stock video options window opens, saves, restores defaults and cancels clean: its CVars are registered, spellEffectLevel drives the particle and shard rates, and the video pairs, frame enumeration and client queries are defined (#565) | pending | pending | - |  |
| 28 | `0f363744` | 2026-09-28 | upstream | The core loads the player's own FrameXML.toc in its order, and the boot VM loads no Fonts.xml (#568) | pending | pending | - |  |
| 29 | `05e204de` | 2026-09-29 | upstream | benilla's layer speaks the 1.12 API alone: handlers take this and no arguments, GetBinding answers the 1.12 list, the Key Bindings page captures through its window, /errors chains the error handler, and the host hooks leave the VM (#569) | pending | pending | - |  |
| 30 | `0a1f6946` | 2026-09-29 | upstream | The nameplate verbs write two runtime bits, cleared at world entry, and the setting lives in FrameXML's saved variables, not two CVars (#571) | pending | pending | - |  |
| 31 | `78a983b0` | 2026-09-29 | upstream | The VM speaks 1.12 alone: the Era globals and the uncalled hooks are gone, the engine's tooltip seams are Rust calls, strata/draw-layer/state-texture/sound/OnClick/quest-item/pickup take their 1.12 shapes, and the surface gate covers the production load (#573) | pending | pending | - |  |
| 32 | `48afcbce` | 2026-09-29 | upstream | The host's clock, zone texts, game clock and error-handler slot leave _G, and the getters over them answer the reference's shapes (#580) | pending | pending | - |  |
| 33 | `33c2c5aa` | 2026-09-29 | upstream | The fourteen missing 1.12.1 functions are defined, the keyring hover runs the stock path, and the stock CloseAllBags keeps a backpack that was open (#581) | pending | pending | - |  |
| 34 | `b8cb8862` | 2026-09-29 | upstream | fix: select the nearest enemy when a combat ability is pressed with no target (#503) | pending | pending | - |  |
| 35 | `459e0e90` | 2026-09-29 | upstream | The stock video window's boxes are registered and ffx, ffxGlow, ffxDeath and gxMaximize work; benilla's options gain Spell Detail; load failures go to FrameXML.log (#585) | pending | pending | - |  |
| 36 | `c574e200` | 2026-09-29 | upstream | fix: drop the spell body from passive talent tooltips (#538) | pending | pending | - |  |
| 37 | `34b7c30a` | 2026-09-29 | upstream | The core loads the chain's Bindings.xml and DefaultBindings.wtf, and every command runs its stock body (#587) | pending | pending | - |  |
| 38 | `5757ab23` | 2026-09-29 | upstream | fix: send no repair-all at a vendor that does not repair (#561) | pending | pending | - |  |
| 39 | `4f86560b` | 2026-09-29 | upstream | fix: spell modifiers (#554) | pending | pending | - |  |
| 40 | `084354c2` | 2026-09-29 | upstream | Spell text keeps "every 1 second": the $l plural keying on every token is recorded as a deviation (#595) | pending | pending | - |  |
| 41 | `91cd43c8` | 2026-09-29 | upstream | fix: apply per-level scaling to spell descriptions (#579) | pending | pending | - |  |
| 42 | `ca7111ed` | 2026-09-29 | upstream | What 1.12.1 lacks lives on top of benilla, which opens a piece of itself to a crate only when its own 1.12.1 work creates it, and loads whatever the stock client loads from a patch (#597) | pending | pending | - |  |
| 43 | `e4b06c5c` | 2026-09-29 | upstream | The battleground queue slots answer "none" from the first frame, so a party login no longer breaks BattlefieldFrame_Update (#603) | pending | pending | - |  |
| 44 | `f000aa01` | 2026-09-29 | upstream | The pet bar's, pet book's and pet-learn trainer's spell tooltips are built against the pet: its level scales the per-level terms, cost and cast time, and its base mana is the percentage cost's basis (#606) | pending | pending | - |  |
| 45 | `d4c280d2` | 2026-09-29 | upstream | Caps Lock remapped to Escape (X11 caps:escape) acts as Escape on Linux, as in the real client (#602) | pending | pending | - |  |
| 46 | `5d25ac92` | 2026-09-29 | upstream | entities: a torn-down net unit gets no new visual work (attach, rebuild, re-dress, re-seat, held items), so no command lands on a despawned unit (#623) | pending | pending | - |  |
| 47 | `b26f7c1f` | 2026-09-29 | upstream | terrain_stream: a placement or WMO prop whose model load failed is retired as resolved, so it stops counting as pending and the loading screen clears (#624) | pending | pending | - |  |
| 48 | `b065554f` | 2026-09-29 | upstream | config.toml saves one quiet second after a change, not only on the exit frame, and a unit test resolves no state folder unless it pins BENILLA_HOME (#625) | pending | pending | - |  |
| 49 | `61f17d46` | 2026-09-29 | upstream | who_sort: the class, race and zone arms order a miss after every resolved name and tie two misses, a total order sort_by can take (#626) | pending | pending | - |  |
| 50 | `45825b06` | 2026-09-29 | upstream | ui_battlefield: a queued battleground's time waited is the time since the status plus the wire's waited ms, computed forwards so no Instant precedes the clock's origin (#628) | pending | pending | - |  |
| 51 | `57f33aa6` | 2026-09-29 | upstream | guild roster: zone and class keys order a miss after every name and NaN days get a place, so the comparator is a total order (#629) | pending | pending | - |  |
| 52 | `6e9e5e99` | 2026-09-29 | upstream | ui_dialog_verbs: a battleground's run time is the time since its status plus the wire's elapsed ms, read forwards so no Instant precedes the clock's origin (#630) | pending | pending | - |  |
| 53 | `06f9cc93` | 2026-09-29 | upstream | char create: the CharBaseInfo guard requires every shipped race/class pair and lets a patch add more, as the class-list builder lists every row (#631) | pending | pending | - |  |
| 54 | `cd441385` | 2026-09-29 | upstream | zone music: a start that plays nothing leaves the pump armed, so the next frame retries with a fresh pick, and a bad path warns once (#632) | pending | pending | - |  |
| 55 | `666300c6` | 2026-09-29 | upstream | doodad anim gate: a meshed host is drawn only while its fade sphere intersects the world camera's frustum, like a meshless one (#633) | pending | pending | - |  |
| 56 | `f768cf3e` | 2026-09-29 | upstream | CastSpellByName's onSelf and CastSpell's third argument cast the spell at the player, the pet book's at the player too, read through GetBoolOrDefault as the reference does (#634) | pending | pending | - |  |
| 57 | `66340800` | 2026-09-29 | upstream | unit tokens: partypetN and raidpetN resolve to the member's held pet, tested before party and raid, and the reach feeds cover them (#635) | pending | pending | - |  |
| 58 | `0e04e54f` | 2026-09-29 | upstream | pet bar: a spell press starts the pet's global cooldown on the pet's list, as 1.12.1's spell arm does before it sends (#636) | pending | pending | - |  |
| 59 | `8ef62da6` | 2026-09-29 | upstream | spell bind: BindTarget's unit gates run ahead of the relation arms at the press, the click and the hover, through one creature-type resolver (#640) | pending | pending | - |  |
| 60 | `e071b8de` | 2026-09-30 | upstream | partypetN and raidpetN unit snapshots: the getters read the held pet or its owner's record, UNIT_PET fires with the owner's token, and the stock party pet frame shows (#644) | pending | pending | - |  |
| 61 | `44fb6e5e` | 2026-09-30 | upstream | spell tooltip feed: each pushed view records what it read, and a change of the player's state rebuilds only the views that read it (#650) | pending | pending | - |  |
| 62 | `4573596e` | 2026-09-30 | upstream | UnitIsConnected answers 1 for every held unit: snapshot sets it, so the pet frame's power bar reads the pet's power instead of a grey maximum (#651) | pending | pending | - |  |
| 63 | `ec5083f4` | 2026-09-30 | upstream | world pick: while a spell targets, a unit or player the word cannot take is no candidate, so the pick reaches the one behind it (0x481050, 0x480610) (#652) | pending | pending | - |  |
| 64 | `3b3ae850` | 2026-09-30 | upstream | spell range: min_max_range takes the unit target and the auto-attack target apart, so a tooltip no longer pads a ranged spell by the swing target's reach (#654) | pending | pending | - |  |
| 65 | `840719fc` | 2026-09-30 | upstream | spell bind: BindTarget's party and raid arms ask 0x606c20 and 0x606d20 and CanAssist, so a group member binds Power Word: Shield, the Blessings and the Seals, and the world pick shares the party predicate (#655) | pending | pending | - |  |
| 66 | `79dc262e` | 2026-09-30 | upstream | pet cooldown list: every mutation fires ACTIONBAR_UPDATE_COOLDOWN, SPELL_UPDATE_COOLDOWN and PET_BAR_UPDATE_COOLDOWN (0x4b31b0, 0x4bce90), and the SMSG_SPELL_GO pet leg no longer signals PET_BAR_UPDATE (#657) | pending | pending | - |  |
| 67 | `3a0728cd` | 2026-09-30 | upstream | ground-targeting cursor: the range verdict takes GetMinMaxRange's bounds, so a range talent (op 5) and the caster's reaches reach it (0x6e6810, 0x6e3480) (#658) | pending | pending | - |  |
| 68 | `6a36d8c3` | 2026-09-30 | upstream | world pick: while no word carries flag 0x20, a unit the active player possesses is no candidate, beside the active player, so the cursor and click reach the unit behind it (0x480610, 0x48064f-0x480674) (#659) | pending | pending | - |  |
| 69 | `9d4b3420` | 2026-09-30 | upstream | pet bar: a spell press with no Spell.dbc record, no active player object or no pet object sends nothing and arms nothing, as the spell arm's exits do (0x4bd2e7-0x4bd34f) (#660) | pending | pending | - |  |
| 70 | `3a613971` | 2026-09-30 | upstream | UnitCreatureType answers the one resolver's type for every unit, so a cat-form druid or a Ghost Wolf shaman is a Beast: the snapshot takes the form, then the template, then the race (0x605570, 0x51a2bc) (#681) | pending | pending | - |  |
| 71 | `47794e48` | 2026-09-30 | upstream | pet cooldown list: Cooldowns::add drops the records that have run out before it appends, so neither list outgrows its live records, and the drop fires no flush (#685) | pending | pending | - |  |
| 72 | `8f6d337a` | 2026-09-30 | upstream | party and raid tokens: UnitIsConnected answers 1 for a held member whatever the roster status byte says, and reads the record's online bit only with no object (0x517d50, 0x517daf, 0x517dd3) (#686) | pending | pending | - |  |
| 73 | `7623cbae` | 2026-09-30 | upstream | pet bar: 0x4bcf70 is one predicate, asked by GetPetActionsUsable, every press and order, Dismiss, both autocast toggles and the drag's writes, so an unusable pet greys the bar and does nothing (#687) | pending | pending | - |  |
| 74 | `79cb3b67` | 2026-09-30 | upstream | PARTY_MEMBERS_CHANGED fires once per SMSG_GROUP_LIST, whatever the list changes, an empty list and an identical one included: GroupState counts the lists and the feed answers each (0x5e6a40, 0x5e6b3c, 0x5e6c61) (#691) | pending | pending | - |  |
| 75 | `602c3038` | 2026-09-30 | upstream | spell range: two running units get GetMinMaxRange's moving bonus (#702) | pending | pending | - |  |
| 76 | `0a920b52` | 2026-09-30 | upstream | unit tokens: party1target and other target chains resolve (#706) | pending | pending | - |  |
| 77 | `4ae99c06` | 2026-09-30 | upstream | raid roster: every raid roster packet fires RAID_ROSTER_UPDATE (#708) | pending | pending | - |  |
| 78 | `5b3c2a98` | 2026-09-30 | upstream | pet bar: PickupPetAction needs the pet in view (#709) | pending | pending | - |  |
| 79 | `2b162193` | 2026-09-30 | upstream | Spell descriptions resolve $b, $f, $F, $i, $q, $u and $v (#622) | pending | pending | - |  |
| 80 | `29c1ec6f` | 2026-09-30 | upstream | party: a roster packet updates each member's online bit (#714) | pending | pending | - |  |
| 81 | `010e41e6` | 2026-09-30 | upstream | unit getters: chained tokens like raid3target answer their unit (#716) | pending | pending | - |  |
| 82 | `c03ca1e4` | 2026-09-30 | upstream | Percentage spell costs use the reference power bases (#646) | pending | pending | - |  |
| 83 | `2b700e6b` | 2026-09-30 | upstream | gates: drop the player build that the player tests already compile (#719) | pending | pending | - |  |
| 84 | `cd34ab77` | 2026-09-30 | upstream | /errors: put Clear and Close in the panel art's sockets (#720) | pending | pending | - |  |
| 85 | `4c18c23c` | 2026-09-30 | upstream | Ambience beds stream from the archive, opened off the main thread (#725) | pending | pending | - |  |
| 86 | `f2e71cce` | 2026-09-30 | upstream | A multi-line EditBox grows with its text; a Lua-made box draws its text (#729) | pending | pending | - |  |
| 87 | `7e117809` | 2026-09-30 | upstream | /errors: the selected error is copyable, in a read-only EditBox (#730) | pending | pending | - |  |
| 88 | `15e83f24` | 2026-09-30 | upstream | Ghost and underwater ambience beds resolve by their reference names (#731) | pending | pending | - |  |
| 89 | `4772489a` | 2026-09-30 | upstream | Arriving bodies composite off the main thread and show once dressed (#732) | pending | pending | - |  |
| 90 | `87f65faf` | 2026-10-01 | upstream | fix: stamp the chat log and FrameXML.log in local time (#723) | pending | pending | - |  |
| 91 | `e9f0e89a` | 2026-10-01 | upstream | EditBox events fire from each box's flush, as 1.12.1 orders them (#734) | pending | pending | - |  |
| 92 | `be9e3d8a` | 2026-10-01 | upstream | Keys are named by the active layout, as 1.12.1 names them (#748) | pending | pending | - |  |
| 93 | `f22ec001` | 2026-10-01 | upstream | EditBox caret keys, selection and focus blink follow 1.12.1 (#750) | pending | pending | - |  |
| 94 | `cfd5fd2c` | 2026-10-02 | upstream | fix: stop shift-click opening the stack splitter on vendor bundles (#751) | pending | pending | - |  |
| 95 | `2ce6a3e8` | 2026-10-02 | upstream | feat: click to move walks to an out-of-reach target, then interacts (#752) | pending | pending | - |  |
| 96 | `e620a0d0` | 2026-10-02 | upstream | feat: let a crate on top interact with an object it picks (#753) | pending | pending | - |  |
| 97 | `4f36c843` | 2026-10-02 | upstream | Emote voices play only for you and your party, as in 1.12.1 (#766) | pending | pending | - |  |
| 98 | `fbea2d9f` | 2026-10-02 | upstream | Emote routing follows 1.12.1's upper-body and mount tables (#767) | pending | pending | - |  |
| 99 | `2dc7d8a9` | 2026-10-02 | upstream | /wave <name> targets the named player; two DoEmote gates follow 1.12.1 (#769) | pending | pending | - |  |
| 100 | `56d8011a` | 2026-10-02 | upstream | NPCs pause their work loop while you talk to them, as in 1.12.1 (#772) | pending | pending | - |  |
| 101 | `4fd0c643` | 2026-10-02 | upstream | One-shot emotes: LiftOff/Land in combat, no restarts, local play (#773) | pending | pending | - |  |
| 102 | `c627483a` | 2026-10-02 | upstream | Return numeric flags from spell-targeting predicates (#764) | pending | pending | - |  |
| 103 | `6c25640a` | 2026-10-02 | upstream | Light bands with out-of-order keys interpolate as a cycle, as in 1.12.1 (#768) | pending | pending | - |  |
| 104 | `98217654` | 2026-10-02 | upstream | A loot window opening on a unit selects it, as in 1.12.1 (#779) | pending | pending | - |  |
| 105 | `b396bbf6` | 2026-10-02 | upstream | Click to Move walks to the ground, along the sky and into melee (#783) | pending | pending | - |  |
| 106 | `cf891dc3` | 2026-10-03 | upstream | fix: close a loot window that opens while you move, as 1.12.1 does (#786) | pending | pending | - |  |
| 107 | `9bc7276d` | 2026-10-04 | upstream | fix: spell text $g takes the player's gender, as 1.12.1 does (#787) | pending | pending | - |  |
| 108 | `4a389add` | 2026-10-04 | upstream | fix: ignore a loot right-click while moving, as 1.12.1 does (#790) | pending | pending | - |  |
| 109 | `aa103d5a` | 2026-10-04 | upstream | fix: level-up gains read signed, so a loss prints -2, as 1.12.1 does (#791) | pending | pending | - |  |
| 110 | `622e1c3e` | 2026-10-04 | upstream | fix: overhead names carry the player's PvP title, as 1.12.1 does (#792) | pending | pending | - |  |
| 111 | `ab909df6` | 2026-10-05 | upstream | fix: TargetUnit("npc") selects the interaction NPC, as 1.12.1 does (#800) | pending | pending | - |  |
| 112 | `bb84f1f1` | 2026-10-05 | upstream | Unit tooltips title the unit with its PvP name, as 1.12.1 does (#802) | pending | pending | - |  |
| 113 | `ffdc206a` | 2026-10-05 | upstream | World-enter queries wait for the player's create, as 1.12.1 sends them (#801) | pending | pending | - |  |
| 114 | `f8493feb` | 2026-10-06 | upstream | The out trace logs every world send, the login handshake's included (#806) | pending | pending | - |  |
| 115 | `85980939` | 2026-10-06 | upstream | A crate on top finds its own benilla-config/ through config_dir (#809) | pending | pending | - |  |
| 116 | `2e82d34c` | 2026-10-06 | upstream | The meeting-stone query runs at every world entry, as 1.12.1 sends it (#810) | pending | pending | - |  |
| 117 | `80b08486` | 2026-10-08 | upstream | fix: trainer services show their spell's description, as 1.12.1 does (#803) | pending | pending | - |  |
| 118 | `63e91796` | 2026-10-08 | upstream | The quest log counts party members on each quest, as 1.12.1 does (#824) | pending | pending | - |  |
| 119 | `ea7e0d08` | 2026-10-08 | upstream | Zone names stay uncoloured on PvE realms outside capitals, as in 1.12.1 (#825) | pending | pending | - |  |
| 120 | `1c5a5681` | 2026-10-08 | upstream | Escape on the realm list closes only the list, as in 1.12.1 (#827) | pending | pending | - |  |
| 121 | `208b1e0b` | 2026-10-08 | upstream | Your own name stays blue or green in FFA areas, as in 1.12.1 (#828) | pending | pending | - |  |
| 122 | `d0d1b5de` | 2026-10-08 | upstream | WORLD_MAP_UPDATE fires inside the verb that moves the map, as in 1.12.1 (#829) | pending | pending | - |  |
| 123 | `87706792` | 2026-10-09 | upstream | A frame or region takes no name or id from its template (#839) | pending | pending | - |  |
| 124 | `f0dfa861` | 2026-10-09 | upstream | PLAYER_LOGIN fires at world entry, once the player exists (#840) | pending | pending | - |  |
| 125 | `dc63a061` | 2026-10-09 | upstream | The UI scale is UIParent's own scale over a 768-unit root (#841) | pending | pending | - |  |
| 126 | `c2fc3cf3` | 2026-10-09 | upstream | OnShow and OnHide fire children first, and after OnLoad, as in 1.12.1 (#842) | pending | pending | - |  |
| 127 | `710eb8f5` | 2026-10-11 | upstream | Dev builds drop ThinLTO; dev and play build dependencies as one unit (#857) | pending | pending | - |  |
| 128 | `45da0a08` | 2026-10-11 | upstream | Optional feature to enable `bevy/dynamic_linking` in crates/benilla-app (#853) | pending | pending | - |  |
| 129 | `ff49eb5b` | 2026-10-11 | upstream | UNIT_FACTION fires on the faction template's own edge (#858) | pending | pending | - |  |
| 130 | `f356e90f` | 2026-10-11 | upstream | Addon files resolve case-insensitively, as on 1.12.1's Windows client (#860) | pending | pending | - |  |
| 131 | `c9ddd803` | 2026-10-11 | upstream | Script errors reach the error handler bare, as 1.12.1 hands them (#865) | pending | pending | - |  |
| 132 | `16affa8d` | 2026-10-11 | upstream | OnUpdate walks strata, level and link order, as 1.12.1 does (#868) | pending | pending | - |  |
| 133 | `c0630f3d` | 2026-10-11 | upstream | check.sh and gates.sh report every failing test, not the first crate's (#869) | pending | pending | - |  |
| 134 | `481cc887` | 2026-10-11 | upstream | A hide drops the press and capture; only a mouse frame's show re-picks (#888) | pending | pending | - |  |
| 135 | `de17e482` | 2026-10-11 | upstream | A level compaction relinks the moved frames in creation order (#891) | pending | pending | - |  |
| 136 | `46512dc5` | 2026-10-11 | upstream | A ScrollFrame builds its ScrollChild before its Frames and OnLoad (#894) | pending | pending | - |  |
| 137 | `07d8c8d4` | 2026-10-11 | upstream | Kind updates run in the frame's walk slot; OnUpdateModel in draw order (#897) | pending | pending | - |  |
| 138 | `a1130c69` | 2026-10-11 | upstream | Mouse disable drops the press; a chorded press goes to the held frame (#899) | pending | pending | - |  |
| 139 | `9be2802f` | 2026-10-11 | upstream | Flag predicates answer 1 or nil, and the gate now sees them all (#904) | pending | pending | - |  |
| 140 | `4e0011bd` | 2026-10-11 | upstream | Backdrop pieces are nine Texture regions on the frame, as in 1.12.1 (#905) | pending | pending | - |  |
| 141 | `b63d6e36` | 2026-10-11 | upstream | A raise's level compaction stops at the stratum's visible high-water (#906) | pending | pending | - |  |
| 142 | `0df67639` | 2026-10-11 | upstream | GameTooltip's fade, owner and health bar follow the 1.12.1 lifecycle (#907) | pending | pending | - |  |
| 143 | `016f08b5` | 2026-10-12 | upstream | A texture handed to a widget's setter moves in on the setter's layer (#910) | pending | pending | - |  |
| 144 | `55729baf` | 2026-10-12 | upstream | Message fades and scroll ranges run in the walk; OnUpdateModel gates (#913) | pending | pending | - |  |
| 145 | `fa5568dc` | 2026-10-12 | upstream | Creating or freeing a frame or region re-derives only what it touches (#915) | pending | pending | - |  |
| 146 | `935b465c` | 2026-10-12 | upstream | Button state textures have one shown bit; widget setters take 1.12 forms (#917) | pending | pending | - |  |
| 147 | `ff3d15b1` | 2026-10-12 | upstream | The tick drains the layout around the walk; size gates in layout units (#922) | pending | pending | - |  |
| 148 | `d65fd190` | 2026-10-12 | upstream | OnClick fires on release while the button is still PUSHED, as in 1.12.1 (#923) | pending | pending | - |  |
| 149 | `eec1804d` | 2026-10-12 | upstream | Chat bubbles are WorldFrame child frames, as in 1.12.1 (#925) | pending | pending | - |  |
| 150 | `b0ad656a` | 2026-10-12 | upstream | A drag release fires OnDragStop and the drop, not OnMouseUp (#928) | pending | pending | - |  |
| 151 | `56f7fa00` | 2026-10-12 | upstream | A resolve keeps a rect that moved under 1.12.1's 1e-5 threshold (#929) | pending | pending | - |  |
| 152 | `a2825a18` | 2026-10-12 | upstream | A second button pressed mid-drag re-arms or keeps it, as in 1.12.1 (#930) | pending | pending | - |  |
| 153 | `af281a05` | 2026-10-12 | upstream | A drag starts at 1% of the screen diagonal, as in 1.12.1 (#931) | pending | pending | - |  |
| 154 | `377542da` | 2026-10-12 | upstream | One anchor slot per point; SetPoint skips an unchanged anchor (#934) | pending | pending | - |  |
| 155 | `6fff0fa7` | 2026-10-12 | upstream | Hover leaves publish the world mouseover loss before the next OnEnter (#935) | pending | pending | - |  |
| 156 | `3bfec890` | 2026-10-12 | upstream | A template name registers once, case-folded, as in 1.12.1 (#937) | pending | pending | - |  |
| 157 | `0fc40d3f` | 2026-10-12 | upstream | An XML state texture installs a fresh region, as in 1.12.1 (#940) | pending | pending | - |  |
| 158 | `73623e31` | 2026-10-12 | upstream | Hover walk: no arg1, held during a move, no fling, published mouseover (#941) | pending | pending | - |  |
| 159 | `87bf5904` | 2026-10-12 | upstream | A region keeps its own name; only the name lookup is first-come (#944) | pending | pending | - |  |
| 160 | `75edfd7c` | 2026-10-12 | upstream | SetFont gates as 1.12.1 does; SetSpacing on every text table (#946) | pending | pending | - |  |
| 161 | `aff59fd5` | 2026-10-12 | upstream | ClearAllPoints keeps the rect; a failed resolve reaches nothing (#950) | pending | pending | - |  |
| 162 | `5af22799` | 2026-10-12 | upstream | A drag pins one TOPLEFT anchor at the frame rect, as in 1.12.1 (#953) | pending | pending | - |  |
| 163 | `108c354f` | 2026-10-12 | upstream | Text measures by the reference kernel: kerned steps, ink tail, no pad (#956) | pending | pending | - |  |
| 164 | `985dabff` | 2026-10-12 | upstream | Layout-mode drag, title press takes the slot, links are their own button (#961) | pending | pending | - |  |
| 165 | `a9dd325b` | 2026-10-12 | upstream | A glue Lua state with the reference glue surface, events and ModelFFX (#963) | pending | pending | - |  |
| 166 | `51a2ce55` | 2026-10-12 | upstream | Layout writes invalidate as 1.12.1 does; only a reached node re-resolves (#964) | pending | pending | - |  |
| 167 | `133414f5` | 2026-10-12 | upstream | WOW_GLUE_STOCK=1 loads and draws the stock GlueXML (#969) | pending | pending | - |  |
| 168 | `45b7914c` | 2026-10-12 | upstream | The stock login screen logs in; realmd 0x0E is a success (#970) | pending | pending | - |  |
| 169 | `7158c585` | 2026-10-12 | upstream | Hidden frames defer their layout; GetWidth answers the set size (#973) | pending | pending | - |  |
| 170 | `a329f19e` | 2026-10-12 | upstream | Reputation lines, NPC greeting gates, and packet bytes in the trace (#976) | pending | pending | - |  |
| 171 | `8ccaa7b4` | 2026-10-12 | upstream | An unattended run keeps its state in a fresh folder of its own (#980) | pending | pending | - |  |
| 172 | `0c58d85c` | 2026-10-12 | upstream | Layout file saves the rect; bubbles re-sort on change; resize Deviation (#981) | pending | pending | - |  |
| 173 | `b2926edf` | 2026-10-12 | upstream | Stock realm list and wizard; realm park cancels cleanly (#985) | pending | pending | - |  |
| 174 | `1efd9a71` | 2026-10-12 | upstream | Lua bindings coerce numbers, flags and colours as 1.12.1 does (#991) | pending | pending | - |  |
| 175 | `1b335717` | 2026-10-12 | upstream | Addon loading and .toc parsing follow AddOn_Load and Toc_Parse (#992) | pending | pending | - |  |
| 176 | `720dcf81` | 2026-10-12 | upstream | Lua bindings return 1/nil and the reference's value counts (#1000) | pending | pending | - |  |
| 177 | `0f505a2e` | 2026-10-12 | upstream | RegisterEvent ignores case; a frame's first resize precedes OnLoad (#1001) | pending | pending | - |  |
| 178 | `24c7cab9` | 2026-10-12 | upstream | Stock character select roster; the screen asks for the list (#1014) | pending | pending | - |  |
| 179 | `aba120cd` | 2026-10-12 | upstream | Lua verbs fire their events before returning, in the reference's order (#1015) | pending | pending | - |  |
| 180 | `d7590081` | 2026-10-12 | upstream | Logout returns to character select on the same world connection (#1022) | pending | pending | - |  |
| 181 | `7547081a` | 2026-10-12 | upstream | fix: a same-map teleport shows no loading screen, as 1.12.1 does (#1018) | pending | pending | - |  |
| 182 | `781bf36a` | 2026-10-12 | upstream | Key input follows the client's walks; stock login takes typing (#1027) | pending | pending | - |  |
| 183 | `bb82dd1a` | 2026-10-12 | upstream | Lua getters answer the reference's miss shapes; tooltip setters' flag (#1030) | pending | pending | - |  |
| 184 | `eb63c886` | 2026-10-12 | upstream | Cursor runs on ClearCursor and the setters' own event tails (#1038) | pending | pending | - |  |
| 185 | `5c7eb5fc` | 2026-10-12 | upstream | Stock glue screens draw their 3-D scenes through the glue booth (#1039) | pending | pending | - |  |
| 186 | `2af91704` | 2026-10-12 | upstream | Stock login fetches the server alert; LaunchURL behind the gate (#1040) | pending | pending | - |  |
| 187 | `79842423` | 2026-10-12 | upstream | Cursor verbs follow the client's arms: bar, bags, doll and trade (#1042) | pending | pending | - |  |
| 188 | `0127b869` | 2026-10-12 | upstream | Stock AddOns screen on the registry; SMSG_ADDON_INFO read in full (#1043) | pending | pending | - |  |
| 189 | `71d65a48` | 2026-10-13 | upstream | Stock character delete and rename, with the client's name check (#1060) | pending | pending | - |  |
| 190 | `e175209f` | 2026-10-13 | upstream | Stock EnterWorld: its guards, the refused login and the glue reseat (#1067) | pending | pending | - |  |
| 191 | `0b4b3ddc` | 2026-10-13 | upstream | Item locks held per item at the client's own lock and unlock sites (#1074) | pending | pending | - |  |
| 192 | `13b7bb50` | 2026-10-13 | upstream | Stock character create: its 24 verbs, state 5 and the booth preview (#1079) | pending | pending | - |  |
| 193 | `d3ac9f75` | 2026-10-13 | upstream | Verbs raise their events in place; feeds push only server changes (#1085) | pending | pending | - |  |
| 194 | `a7f331ef` | 2026-10-13 | upstream | Item use follows the client: bag store, bar equip, BoE hold, control (#1086) | pending | pending | - |  |
| 195 | `39d1753a` | 2026-10-13 | upstream | Glue verbs run inside their Lua call; Update arms signal before writing (#1088) | pending | pending | - |  |
| 196 | `48a62ee8` | 2026-10-13 | upstream | A scroll child draws inside its ScrollFrame's highlight callback (#1090) | pending | pending | - |  |
| 197 | `11f6fe97` | 2026-10-13 | upstream | Stock glue: credits music, cursor hide, status number, Model pane lights (#1091) | pending | pending | - |  |
| 198 | `8537ce7c` | 2026-10-13 | upstream | Forced reactions answer both reaction directions and refresh unit frames (#1096) | pending | pending | - |  |
| 199 | `d106c124` | 2026-10-13 | upstream | Glue booth keeps its model: create re-dresses, a hidden scene pauses (#1097) | pending | pending | - |  |
| 200 | `5a93d1a4` | 2026-10-13 | upstream | Gate failures print the failing test before the log tail (#1098) | pending | pending | - |  |
| 201 | `c77386c4` | 2026-10-13 | upstream | Leaving the world closes every window in the client's sweep order (#1100) | pending | pending | - |  |
| 202 | `a96977e7` | 2026-10-13 | upstream | Gradients tint per vertex; a layer's textures sort by handle (#1101) | pending | pending | - |  |
| 203 | `dd3983a4` | 2026-10-13 | upstream | Lua API edges: tradeskill filters, rolls, autocast, bindings and mail (#1102) | pending | pending | - |  |
| 204 | `537defc7` | 2026-10-13 | upstream | Link benilla-re, the reverse engineering benilla is built against (#1141) | pending | pending | - |  |
| 205 | `5338675f` | 2026-10-13 | upstream | EditBoxes keep the client's text window; paste types per character (#1143) | pending | pending | - |  |
| 206 | `b15fb9ab` | 2026-10-14 | upstream | A MessageFrame's link is text; only chat lines take link buttons (#1149) | pending | pending | - |  |
| 207 | `94eb6a8b` | 2026-10-14 | upstream | TargetByName, TargetUnit and ClearTarget show their pick in the chunk (#1244) | pending | pending | - |  |
| 208 | `77b05e34` | 2026-10-15 | upstream | Docs: find the reference fact in benilla-re, and take its gaps there (#1280) | pending | pending | - |  |
| 209 | `994f9f96` | 2026-10-15 | upstream | reference/ vendors benilla-re's tables at a pinned commit (#1282) | pending | pending | - |  |
| 210 | `b7bc7a7e` | 2026-10-15 | upstream | Contributor docs: the quick test loop, the reference pin, open gaps (#1283) | pending | pending | - |  |
| 211 | `8f6162e5` | 2026-10-15 | upstream | UI core: hard line breaks, loose files first, tiling, chat link buttons (#1285) | pending | pending | - |  |
| 212 | `c49c2011` | 2026-10-15 | upstream | CVars refuse what 1.12.1's callbacks refuse; Lua arguments and numerals (#1286) | pending | pending | - |  |
| 213 | `e5321bed` | 2026-10-15 | upstream | NPC windows share 1.12.1's interaction latch, trade and books included (#1290) | pending | pending | - |  |
| 214 | `f5dd4c79` | 2026-10-15 | upstream | The gates memo treats the issue and PR templates as docs (#1293) | pending | pending | - |  |
| 215 | `358731f9` | 2026-10-15 | upstream | FontStrings wrap by 1.12.1's line kernel (#1306) | pending | pending | - |  |
| 216 | `cf1aa41e` | 2026-10-15 | upstream | NPC greetings from the latch; death and shapeshift close the window (#1320) | pending | pending | - |  |
| 217 | `b21b49c0` | 2026-10-15 | upstream | The 44 Lua functions 1.12.1 registers that benilla lacked (#1322) | pending | pending | - |  |
| 218 | `b816ff2f` | 2026-10-15 | upstream | Update packets: self release, mover blocks, spline facing, placement (#1324) | pending | pending | - |  |
| 219 | `5b12d4f2` | 2026-10-15 | upstream | The in-flight cast slot is pushed and popped where 1.12.1 does (#1330) | pending | pending | - |  |
| 220 | `78e8c27a` | 2026-10-15 | upstream | Event gates read every fire site; five NPC events fire bare (#1336) | pending | pending | - |  |
| 221 | `9c6c03de` | 2026-10-15 | upstream | A held item's emitters ride its idle-posed bones, in the world and the booths (#1334) | pending | pending | - |  |
| 222 | `e3eade8e` | 2026-10-15 | upstream | Lua errors and chat gates where 1.12.1 has them (#1339) | pending | pending | - |  |
| 223 | `4ab54ad3` | 2026-10-15 | upstream | Loose install files answer the opens 1.12.1 flags (#1342) | pending | pending | - |  |
| 224 | `623c149f` | 2026-10-15 | upstream | docs/DEVIATIONS.md: where benilla differs from 1.12.1, generated (#1345) | pending | pending | - |  |
| 225 | `14ae7226` | 2026-10-15 | upstream | Cooldown reads, GCD after an event, lost control as 1.12.1 has them (#1348) | pending | pending | - |  |
| 226 | `20775b15` | 2026-10-15 | upstream | Tooltip lines, EditBox byte caps and slot textures as 1.12.1 has them (#1350) | pending | pending | - |  |
| 227 | `e6f9d93d` | 2026-10-15 | upstream | The Warden deviation says why benilla cannot answer it (#1351) | pending | pending | - |  |
| 228 | `b727e30f` | 2026-10-15 | upstream | Character model frames draw their own SetUnit subject (#1352) | pending | pending | - |  |
| 229 | `b220d6a4` | 2026-10-16 | upstream | Debug globals, talent icons, European numbers, the protected-action gate (#1353) | pending | pending | - |  |
| 230 | `831e4d3a` | 2026-10-16 | upstream | The client reads its locale from Wow.ini, as 1.12.1 does (#1360) | pending | pending | - |  |
| 231 | `7b608671` | 2026-10-16 | upstream | Item objects notify the action bar and trade as 1.12.1's handlers do (#1366) | pending | pending | - |  |
| 232 | `93e50421` | 2026-10-16 | upstream | Reference at c854c4e; player-field events carry their watch's arguments (#1386) | pending | pending | - |  |
| 233 | `99458b32` | 2026-10-16 | upstream | Protected calls pass tainted code only inside an input event (#1388) | pending | pending | - |  |
| 234 | `8c358747` | 2026-10-16 | upstream | The client dials the first IPv4 address alone, as 1.12.1 does (#1385) | pending | pending | - |  |
| 235 | `407aeed5` | 2026-10-16 | upstream | Item counts walk mask 0; trade state ends and signals as 1.12.1's (#1395) | pending | pending | - |  |
| 236 | `7b5cfef2` | 2026-10-16 | upstream | Field-watch events fire per values block, never on create or reload (#1396) | pending | pending | - |  |
| 237 | `4332bcb9` | 2026-10-16 | upstream | Every target select runs SelectTarget's charmed and spell-cursor front (#1400) | pending | pending | - |  |
| 238 | `4e84da43` | 2026-10-16 | upstream | Names, words, fonts and defaults follow the client's locale (#1407) | pending | pending | - |  |
| 239 | `7923d67a` | 2026-10-16 | upstream | A new nameplate draws its first WORLD batch in the frame it's built (#1341) | pending | pending | - |  |
| 240 | `5c198446` | 2026-10-16 | upstream | README: any 1.12.1 client locale reads its own text (#1387) | pending | pending | - |  |
| 241 | `330b2976` | 2026-10-16 | upstream | The loose-file index lists only the folders a lookup reaches (#1413) | pending | pending | - |  |
| 242 | `eb6bb9d9` | 2026-10-16 | upstream | Unit events fire from field watches and packets, not a per-frame diff (#1415) | pending | pending | - |  |
| 243 | `83bbf55d` | 2026-10-16 | upstream | Objects are read by their descriptor's entry, and three fork-found fixes (#1421) | pending | pending | - |  |
| 244 | `740bd27a` | 2026-10-16 | upstream | The wheel walks the wheel list once, and the world takes what is left (#1422) | pending | pending | - |  |
| 245 | `caccf1be` | 2026-10-16 | upstream | Stock glue: live scene after the world, reloadUI rebuilds, header width (#1426) | pending | pending | - |  |
| 246 | `106e7280` | 2026-10-16 | upstream | A texture's wrap is fixed when its file loads, not read from its UVs (#1427) | pending | pending | - |  |
| 247 | `03e4ba5e` | 2026-10-16 | upstream | Record the unstreamed-creature guid read as a deviation (#1428) | pending | pending | - |  |
| 248 | `2027a705` | 2026-10-16 | upstream | EditBox caret, Ctrl editing letters and word rule follow 1.12.1 (#1430) | pending | pending | - |  |
| 249 | `4a76d48f` | 2026-10-16 | upstream | Two UI tests read the files and frames they claim to check (#1431) | pending | pending | - |  |
| 250 | `c7b4898b` | 2026-10-16 | upstream | Buff anchors and the minimap mask follow 1.12.1's load order (#1432) | pending | pending | - |  |
| 251 | `5d19a7be` | 2026-10-16 | upstream | Stock glue: raw realm preferences, queue countdown, empty-roster smokes (#1438) | pending | pending | - |  |
| 252 | `951f9fc4` | 2026-10-16 | upstream | Glue session state serves both screen paths; scripted runs pick a realm (#1443) | pending | pending | - |  |
| 253 | `8a545a66` | 2026-10-16 | upstream | Tooltips colour Locked by the lock check and show Cooldown remaining (#1444) | pending | pending | - |  |
| 254 | `902e170a` | 2026-10-16 | upstream | The roster, the pick and the session edges move into glue_session (#1446) | pending | pending | - |  |
| 255 | `ff3dbc94` | 2026-10-16 | upstream | Worn items answer their cooldown; a spent charged item drops it (#1447) | pending | pending | - |  |
| 256 | `9ff8f6af` | 2026-10-16 | upstream | The login engine moves into glue_session; native login shows its notices (#1451) | pending | pending | - |  |
| 257 | `bc52e8a7` | 2026-10-16 | upstream | Item tooltips show the lock's key or skill line and a gift's giver (#1454) | pending | pending | - |  |
| 258 | `d1fa9bde` | 2026-10-16 | upstream | Quest givers streamed in before our own tag are still asked their status (#1455) | pending | pending | - |  |
| 259 | `247ae3e6` | 2026-10-16 | upstream | Nothing of the native glue screens runs under the stock screens (#1456) | pending | pending | - |  |
| 260 | `2d22328b` | 2026-10-16 | upstream | A schedule census keeps every self-reading system after the self tag (#1457) | pending | pending | - |  |
| 261 | `d6655d8c` | 2026-10-16 | upstream | benilla-lua: benilla's own 1.12 Lua VM, opt-in behind benilla-vm (#1254) | pending | pending | - |  |
| 262 | `a9c2672b` | 2026-10-16 | upstream | A fifth loot roll waits for a frame and opens when one closes (#1458) | pending | pending | - |  |
| 263 | `d404e4e9` | 2026-10-16 | upstream | The stock glue screens are the default; native ones are dev-only (#1462) | pending | pending | - |  |
| 264 | `baf729ce` | 2026-10-16 | upstream | Nameplates read NAMEPLATE_FONT when built, as 1.12.1 does (#1464) | pending | pending | - |  |
| 265 | `129b3bb8` | 2026-10-16 | upstream | GetTradeSkillNumMade answers 1, 1 on a header or past the rows (#1459) | pending | pending | - |  |
| 266 | `c0fa6561` | 2026-10-16 | upstream | The native glue screens are gone; instruments press the stock screens (#1466) | pending | pending | - |  |
| 267 | `43dfb27a` | 2026-10-16 | upstream | GlobalStrings reads no longer hold the UI model borrow (#1467) | pending | pending | - |  |
| 268 | `c87e26ad` | 2026-10-16 | upstream | Addon-harness tests link the corpus in a way that builds on Windows (#1468) | pending | pending | - |  |
| 269 | `5d577c27` | 2026-10-16 | upstream | A FontString with no font raises on SetText, measures 0, draws nothing (#1470) | pending | pending | - |  |
| 270 | `c50db9bf` | 2026-10-16 | upstream | A player-clippy gate lints the shipped player build (#1471) | pending | pending | - |  |
| 271 | `7158cd1e` | 2026-10-16 | upstream | XML font= with no positive FontHeight warns and sets no font (#1488) | pending | pending | - |  |
| 272 | `d8720510` | 2026-10-16 | upstream | A worktree with its own probe account keeps its own state folder (#1489) | pending | pending | - |  |
| 273 | `91803130` | 2026-10-16 | upstream | Item tooltips rebuild in place when a cache they asked answers (#1515) | pending | pending | - |  |
| 274 | `70635496` | 2026-10-16 | upstream | Character select notices a server that drops the link (#1563) | pending | pending | - |  |
| 275 | `f438c19f` | 2026-10-16 | upstream | Quest item toasts come from their packet, not from bag-count changes (#1606) | pending | pending | - |  |
| 276 | `6f653107` | 2026-10-16 | upstream | A boot with no readable WoW install is fatal, with an error dialog (#1630) | pending | pending | - |  |
| 277 | `87f3b24f` | 2026-10-16 | upstream | A refused shot no longer chains Auto Shot (#1638) | pending | pending | - |  |
| 278 | `cda3a91f` | 2026-10-17 | upstream | Instances built as one WMO take their zone from Map.dbc (#1670) | pending | pending | - |  |
| 279 | `fd642f79` | 2026-10-17 | upstream | Print screen on the glue screens takes a screenshot (#1673) | pending | pending | - |  |
| 280 | `ec971695` | 2026-10-17 | upstream | SPELLCAST_STOP fires from the cast result, for every kind of cast (#1704) | pending | pending | - |  |
| 281 | `71321043` | 2026-10-17 | upstream | A dead player or ghost cannot use an object; its cursor grays (#1709) | pending | pending | - |  |
| 282 | `9e1b3e9a` | 2026-10-17 | upstream | cameraYawSmoothSpeed follows at the stored rate, not the slider's 90-270 (#1712) | pending | pending | - |  |
| 283 | `a8ad08a3` | 2026-10-17 | upstream | A refused cache answer is freed, never saved, and asked again (#1713) | pending | pending | - |  |
| 284 | `67345f99` | 2026-10-17 | upstream | The world map lights and names the continent under the cursor (#1715) | pending | pending | - |  |
| 285 | `112601c2` | 2026-10-17 | upstream | The loading screen fits its picture inside the window both ways (#1716) | pending | pending | - |  |
| 286 | `52cc825e` | 2026-10-17 | upstream | A server's Warden request is read past and the login goes on (#1717) | pending | pending | - |  |
| 287 | `65a9016d` | 2026-10-17 | upstream | A server's billing minutes give the 1.12 play-time warning (#1718) | pending | pending | - |  |
| 288 | `1c6d4a71` | 2026-10-17 | upstream | A still unit plays its aura state kit's animation, as Lazy Peons sleep (#1721) | pending | pending | - |  |
| 289 | `f24320c3` | 2026-10-17 | upstream | Chat filters use the client's word and space classes, not Unicode's (#1729) | pending | pending | - |  |
| 290 | `142ed718` | 2026-10-17 | upstream | The Benilla scroll bar template scrolls its parent itself (#1730) | pending | pending | - |  |
| 291 | `0d7c4f06` | 2026-10-17 | upstream | Billing docs name plan bit 3 the Internet Gaming Room (IGR) plan (#1731) | pending | pending | - |  |
| 292 | `7da1f7e7` | 2026-10-17 | upstream | README: any 1.12.1 server runs benilla; Warden is the server's policy (#1733) | pending | pending | - |  |
| 293 | `5f96e65d` | 2026-10-17 | upstream | Two-texture M2 batches draw their env-mapped second stage (#1734) | pending | pending | - |  |
| 294 | `251364db` | 2026-10-17 | upstream | A fading opaque M2 batch fades evenly, not by its texture's alpha (#1735) | pending | pending | - |  |
| 295 | `f5992d00` | 2026-10-17 | upstream | A character its font lacks draws nothing, not a box (#1736) | pending | pending | - |  |
| 296 | `8100fe98` | 2026-10-17 | upstream | Run the script host on benilla-lua; remove the vendored C Lua and mlua (#1255) | pending | pending | - |  |
| 297 | `a64b6b8a` | 2026-10-17 | upstream | A kit's Stand animation plays as in 1.12, with its two driver rules (#1739) | pending | pending | - |  |
| 298 | `a04df2d1` | 2026-10-17 | upstream | benilla-lua's VM and compiler behave as the client's Lua 5.0 (#1768) | pending | pending | - |  |
| 299 | `b3bfae15` | 2026-10-17 | upstream | Cache readers ask at the client's trigger or only read; no timed re-ask (#1772) | pending | pending | - |  |
| 300 | `826194e2` | 2026-10-17 | upstream | benilla-lua's strings and numbers read and print as the client's (#1773) | pending | pending | - |  |
| 301 | `d5e43dd3` | 2026-10-17 | upstream | Lua's table, bit and error paths behave as the client's 5.0 (#1774) | pending | pending | - |  |
| 302 | `1764e9b0` | 2026-10-17 | upstream | Every chosen difference from 1.12.1 carries its Deviation: line (#1775) | pending | pending | - |  |
| 303 | `9ef68a30` | 2026-10-17 | upstream | Workspace tests pass on a Windows checkout's paths and line ends (#1776) | pending | pending | - |  |
| 304 | `772fbacb` | 2026-10-17 | upstream | benilla-app: hot reload, an addon profiler and a Lua inspector (dev) (#1256) | pending | pending | - |  |
| 305 | `cd46d2ee` | 2026-10-17 | upstream | benilla-lua's lexer and compiler behave as the client's Lua 5.0 (#1781) | pending | pending | - |  |
| 306 | `167ae468` | 2026-10-17 | upstream | The script memory limit warns, then quits, as 1.12.1 does (#1782) | pending | pending | - |  |
| 307 | `d93c231d` | 2026-10-17 | upstream | Linux audio plays: cpal 0.17.3 fixes ALSA's stalled callback (#1785) | pending | pending | - |  |
| 308 | `80f751ff` | 2026-10-17 | upstream | Lua memory counts as the client's 5.0; the old shim's overrides are gone (#1788) | pending | pending | - |  |
| 309 | `d8a55240` | 2026-10-17 | upstream | Lua's collector paces and sizes its stack as the client's 5.0 (#1791) | pending | pending | - |  |
| 310 | `c97fd03a` | 2026-10-17 | upstream | Lua tables, ipairs and globals behave as the client's 5.0 (#1792) | pending | pending | - |  |
| 311 | `93098a06` | 2026-10-17 | upstream | Chat frames lay out, page and scroll as 1.12.1's; panes aim as they open (#1799) | pending | pending | - |  |
| 312 | `9eed3d5b` | 2026-10-17 | upstream | The quest log reads live and signals as 1.12.1's does (#1801) | pending | pending | - |  |
| 313 | `b817db8d` | 2026-10-17 | upstream | The fake world counts a session before its auth reply (#1802) | pending | pending | - |  |
| 314 | `746ad351` | 2026-10-17 | upstream | Chat frames fade, hold their view and space messages as 1.12.1's (#1803) | pending | pending | - |  |
| 315 | `b2473e60` | 2026-10-17 | upstream | Quest completion and timed-quest failure follow 1.12.1 (#1805) | pending | pending | - |  |
| 316 | `bb434a18` | 2026-10-17 | upstream | The ESC menu's Support rung opens the help window (#1807) | pending | pending | - |  |
| 317 | `1e28153a` | 2026-10-17 | upstream | Sound plays on another output when ALSA's default will not open (#1808) | pending | pending | - |  |
| 318 | `86c71919` | 2026-10-17 | upstream | Login notices show after the loading screen, as in 1.12.1 (#1809) | pending | pending | - |  |
| 319 | `8a6c7403` | 2026-10-17 | upstream | Sound on Linux skips ALSA's null output when the default fails (#1810) | pending | pending | - |  |
| 320 | `0117b595` | 2026-10-17 | upstream | The loading-screen tip is drawn by the font engine, as in 1.12.1 (#1814) | pending | pending | - |  |
| 321 | `560d7ff6` | 2026-10-18 | upstream | A jump on a moving lift measures its fall in the lift's frame (#1818) | pending | pending | - |  |
| 322 | `f55bb79b` | 2026-10-18 | upstream | Monochrome fonts draw 1-bit with their ring, as in 1.12.1 (#1821) | pending | pending | - |  |
| 323 | `8731395b` | 2026-10-18 | upstream | Outrunning terrain streaming stalls the frame, not a loading screen (#1824) | pending | pending | - |  |
| 324 | `f6d8b162` | 2026-10-18 | upstream | Text glyphs rasterize unhinted at the client's seat, as in 1.12.1 (#1826) | pending | pending | - |  |
| 325 | `cff056c3` | 2026-10-18 | upstream | Glyph cells clip to the em and keep 4-bit alpha, as in 1.12.1 (#1829) | pending | pending | - |  |
| 326 | `72c53e1d` | 2026-10-18 | upstream | The quest log shows a quest's event and reputation lines (#1840) | pending | pending | - |  |
| 327 | `f199fbd3` | 2026-10-18 | upstream | The world's edge holds the player, as in 1.12.1 (#1841) | pending | pending | - |  |
| 328 | `3e6d9cd1` | 2026-10-18 | upstream | The glue screens box at 16:9 on a window wider than that (#1848) | pending | pending | - |  |
| 329 | `30f2955e` | 2026-10-18 | upstream | SetCamera on a character pane draws it through the camera it names (#1849) | pending | pending | - |  |
| 330 | `03f192f2` | 2026-10-18 | upstream | RegisterAllEvents joins every event's list in registration order (#1850) | pending | pending | - |  |
| 331 | `81fd090e` | 2026-10-18 | upstream | Saved variables are read as 1.12.1 reads them (#1857) | pending | pending | - |  |
| 332 | `615198d3` | 2026-10-18 | upstream | The Lua library answers as 1.12.1's does (#1862) | pending | pending | - |  |
| 333 | `94fb1185` | 2026-10-18 | upstream | The UI ticks on the paced frame step the world gets (#1870) | pending | pending | - |  |
| 334 | `9e55f481` | 2026-10-18 | upstream | A pane's blend slot keeps the outgoing sequence on its own clock (#1881) | pending | pending | - |  |
| 335 | `20abc746` | 2026-10-18 | upstream | GameTooltip sizes in the show, as 1.12.1 does (#1884) | pending | pending | - |  |
| 336 | `31ff8ec0` | 2026-10-18 | upstream | The XML loader reads each template level, as 1.12.1 does (#1892) | pending | pending | - |  |
| 337 | `39ad58c7` | 2026-10-18 | upstream | Buttons click, check and press their label as 1.12.1's do (#1893) | pending | pending | - |  |
| 338 | `ba96e0cc` | 2026-10-18 | upstream | The CVar set path is 1.12.1's InternalSet (#1896) | pending | pending | - |  |
| 339 | `39311b2f` | 2026-10-18 | upstream | Fonts link live to their parent, as 1.12.1's do (#1901) | pending | pending | - |  |
| 340 | `ed630ffb` | 2026-10-18 | upstream | IsRaidOfficer answers 1 for an assistant or the leader, as 1.12.1 does (#1894) | pending | pending | - |  |
| 341 | `dd61ea73` | 2026-10-18 | upstream | Textures load, answer and blend as 1.12.1's do (#1902) | pending | pending | - |  |
| 342 | `ecb49a5f` | 2026-10-18 | upstream | A unit the player cannot select shows no name, as 1.12.1 does (#1895) | pending | pending | - |  |
| 343 | `2f9dd94a` | 2026-10-18 | upstream | RAID_TARGET_UPDATE fires after the unit snapshots, so a new mark reaches the target frame at once (#1897) | pending | pending | - |  |
| 344 | `ed651455` | 2026-10-18 | upstream | The addon probe fires events, clicks and runs on the bare interface (#1898) | pending | pending | - |  |
| 345 | `7940c2bd` | 2026-10-18 | upstream | Colours and alphas are bytes, and frame verbs gate as 1.12.1's do (#1907) | pending | pending | - |  |
| 346 | `d80200ed` | 2026-10-18 | upstream | The addon harness seats its target at world entry, so TargetFrame loads (#1900) | pending | pending | - |  |
| 347 | `a5cb73a8` | 2026-10-18 | upstream | CVar reset, cvarlist, saving and console text follow 1.12.1 (#1931) | pending | pending | - |  |
| 348 | `cc16abfa` | 2026-10-18 | upstream | Glue scenes stand any file; one model-name rule for every lane (#1932) | pending | pending | - |  |
| 349 | `01157149` | 2026-10-18 | upstream | The macro icon list is 1.12.1's, built on first use (#1934) | pending | pending | - |  |
| 350 | `3acad6b1` | 2026-10-18 | upstream | UI usage lines, argument gates and number text follow 1.12.1 (#1938) | pending | pending | - |  |
| 351 | `2bfbf19e` | 2026-10-18 | upstream | XML font legs and font-object getters follow 1.12.1 (#1940) | pending | pending | - |  |
| 352 | `d463c0f0` | 2026-10-18 | upstream | Minimap state, zoom, ping, mask and model arrows follow 1.12.1 (#1941) | pending | pending | - |  |
| 353 | `e0031ab9` | 2026-10-18 | upstream | A tooltip line wrapped past 20 rows sizes to its pieces, as in 1.12.1 (#1933) | pending | pending | - |  |
| 354 | `837a6e13` | 2026-10-18 | upstream | Frame sizes and positions follow 1.12.1's screen-unit law (#1946) | pending | pending | - |  |
| 355 | `fdbb4bb0` | 2026-10-18 | upstream | GetTrainerServiceInfo answers "" for a rankless service's subText (#1947) | pending | pending | - |  |
| 356 | `57384551` | 2026-10-18 | upstream | Widget methods, chat-frame state and line fades follow 1.12.1 (#1949) | pending | pending | - |  |
| 357 | `7ab0596a` | 2026-10-18 | upstream | GetAuctionItemInfo answers the miss shape for a row not yet cached (#1950) | pending | pending | - |  |
| 358 | `817ce799` | 2026-10-19 | upstream | XML anchors, numbers, lengths, shadows and frame elements follow 1.12.1 (#1951) | pending | pending | - |  |
| 359 | `1c0b8dad` | 2026-10-19 | upstream | SimpleHTML font nodes, direct FontStrings and pane cameras follow 1.12.1 (#1957) | pending | pending | - |  |
| 360 | `d9926c6b` | 2026-10-19 | upstream | StatusBar bits, index usage lines, stack names and cursors follow 1.12.1 (#1959) | pending | pending | - |  |
| 361 | `e550df4b` | 2026-10-19 | upstream | Model frames load their own file again, not the extension-less key (#1961) | pending | pending | - |  |
| 362 | `b2387b27` | 2026-10-19 | upstream | Resize, inset, backdrop, font and XML lengths follow the screen-unit law (#1962) | pending | pending | - |  |
| 363 | `8e5114d8` | 2026-10-19 | upstream | Setters, bindings and loot methods gate their arguments as 1.12.1 does (#1963) | pending | pending | - |  |
| 364 | `4dc5f1c1` | 2026-10-19 | upstream | Every by-name lookup runs 1.12.1's one resolver (#1966) | pending | pending | - |  |
| 365 | `88eaf167` | 2026-10-19 | upstream | GetContainerNumSlots and GetBagName answer on 1.12.1's bag ranges (#1968) | pending | pending | - |  |
| 366 | `866e2cac` | 2026-10-19 | upstream | A nameplate's health fill shrinks with the unit's health again (#1970) | pending | pending | - |  |
| 367 | `881152bd` | 2026-10-19 | upstream | Key-binding confirm dialogs no longer raise a Lua error each frame (#1972) | pending | pending | - |  |
| 368 | `f52df3a1` | 2026-10-19 | upstream | A key moved off an action button takes its label along (#1974) | pending | pending | - |  |
| 369 | `05e7cf4b` | 2026-10-19 | upstream | A level-60 character shows the max-level bar, not the XP bar (#1976) | pending | pending | - |  |
| 370 | `cda0affa` | 2026-10-19 | upstream | UnitDebuff's dispelType is SpellDispelType's column 11 on every locale, as 1.12.1 returns it (#1975) | pending | pending | - |  |
| 371 | `9969b845` | 2026-10-19 | upstream | A moved bar item takes its green border along and leaves none behind (#1977) | pending | pending | - |  |
| 372 | `3503592a` | 2026-10-19 | fork | Quake 3 movement behind the q3_movement feature (off by default) | pending | pending | - |  |
| 373 | `9bfa6245` | 2026-10-19 | merge | Merge branch 'main' of https://github.com/ornfelt/benilla | skipped (merge, changes nothing) | skipped (merge, changes nothing) | as-is | --remerge-diff and --cc empty |
| 374 | `9a5f2a56` | 2026-10-19 | upstream | Release 0.3.0 (#1979) | pending | pending | - |  |
| 375 | `daf1bc9b` | 2026-10-19 | merge | Merge branch 'samwhosung:main' into main | skipped (merge, changes nothing) | skipped (merge, changes nothing) | as-is | --remerge-diff and --cc empty |

## Special handling

- #14 `8d31e9e9` A crate on top of benilla adds its own Bevy plugins through `benilla_app::run_with` (#495) - gfx-dll-backend: merged as is (its root manifest patches the same two crates as upstream's, so the example's documented manifest and `launcher_shape` hold unchanged). trimmed-bevy: the `lib.rs` conflict keeps the branch's deletion of the `ExtractSchedule` executor flip (a render-world piece trimmed-bevy has no render app for) and takes the new `extend` hook in the same place, before the probe fleet; the example's documented manifest is adapted, because its new `the_documented_launcher_repeats_the_root_patches_profile_and_bevy` test holds it to the root's `[patch.crates-io]`, which on trimmed-bevy also patches the 50 vendored Bevy, avian and egui crates: the header lists all 52 patches, and its benilla git sources name `ornfelt/benilla` branch `trimmed-bevy`, as a crate on top of this branch would take them (without them Bevy is crates.io's, wgpu and winit back). Doc comment only; the code is upstream's.

## Flagged for upstream

None yet.

## Open questions

None yet.
