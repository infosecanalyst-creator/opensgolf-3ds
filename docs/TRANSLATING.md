# Translating OpenSGolf

Every text the game shows comes from a string table, in the OpenTTD style. The English table, `lang/en.txt`, is built
into the program; a translation is a second file with the same keys and the text in another language. A key the
translation does not have shows in English, and a key nobody has shows as the key itself (so a missing text is easy to
spot).

Text that comes from the player's own game files (the golfer stories, the characters' names and sayings, the credits, the
terrain descriptions in `Interface/<theme>.txt`, the celebrities and pro golfers lists) is not in the table: it shows as
the files have it.

## The file

A translation is a UTF-8 text file named after its language code, `lang/<code>.txt`: `de.txt`, `fr.txt`, `pt_BR.txt`
(letters, digits, `-` and `_`). Start from a copy of `lang/en.txt`:

```
##language de
##name Deutsch
#
# comments start with #
STR_MSG_CHALLENGE                       :{PRO} fordert {GOLFER} zu einem Match heraus!
STR_TOAST_NOT_ENOUGH_MONEY              :Du hast nicht genug Geld.
```

* `##language <code>` and `##name <name>` at the top: the code and the name the Preferences show for the language.
* One text per line: the key, any spaces, a colon, then the text. **The text is everything after the colon, spaces
  included.** Some English texts start or end with a space because the game joins them with other pieces (the
  remarkable-shot message is built from about ten of them); keep those spaces where your language needs them. Your editor
  must not trim spaces at the ends of lines (the repository's `.editorconfig` says so for `lang/*.txt`).
* The options of a pop-up box (Information, System Functions, Preferences, the quit question, the land offer, ...) are the
  lines whose text starts with a space; the other lines are its headings. Keep the leading space on the options.
* Escapes: `\n` a line break, `\t` a tab, `\\` a backslash, `\{` and `\}` braces that are not a parameter.
* Lines starting with `#` and blank lines are ignored. You do not need to keep the order or the comments, but keeping the
  English file's order makes it easy to compare.
* Leave out what you have not translated yet: those texts stay English.

## Parameters

Names in braces are filled in by the game: `{GOLFER}` a golfer's name, `{PRO}` the player's pro (or a famous golfer),
`{MONEY}` an amount (as digits: the money sign is in the text, `§{MONEY}`), `{HOLE}` a hole number, `{YEAR}`, `{COURSE}`,
`{TERRAIN}`, `{BUILDING}` and so on; the comment above each group of texts says what its parameters are. Use them in the
order your language needs and leave out none: a parameter your text does not name is simply not shown. A few texts take
positions instead of names (`{0}`, `{1}`), the same way.

Where the English depends on a number or a person, the game has a key for each form: `STR_MATCH_LEADS_BY_ONE` and
`STR_MATCH_LEADS_BY` (one shot, several shots), `STR_NARR_HE` and `STR_NARR_SHE`, `STR_MSG_HIS` and `STR_MSG_HER`. Some
English sentences are made of pieces the original game puts together (a golfer's quit message is `{GOLFER} {REASON}
{WHEN}.`, a hole's opening announcement is seven pieces); translate the pieces so that they read well together.

## Trying it out

* Put the file in a `lang` folder in the save folder (`~/.local/share/opensgolf/lang/` on Linux,
  `%APPDATA%\OpenSGolf\lang\` on Windows, `~/Library/Application Support/OpenSGolf/lang/` on macOS), next to the program,
  or in the folder you start the game from. Packs (docs/MODDING.md) can carry `lang/<code>.txt` too.
* Choose the language: once a translation is there, **System Functions > Preferences** shows a `Language:` line; each
  click switches to the next language and remembers it in `settings.json` (`"language": "de"`). You can also write the
  setting yourself, or start the game with `--lang de` for one run.
* `--lang-file path/to/de.txt` starts the game with that file over the English, wherever it is, which is handy while you
  work on it.
* `SG_TEXTLOG=texts.txt` (an environment variable) writes every text the game draws or posts to the message window to
  `texts.txt`, each different text once. Together with the screen test hooks of docs/PLAYING.md (`--screen report`,
  `--png out.png`, ...) it shows which texts you still have to do; a pseudo translation (every text in `[[ ]]`) shows any
  text that does not come from the table at all.

## For programmers

The table is `sg_core::lang`: `tr("STR_KEY")` gives a text (a `&'static str`), `trf("STR_KEY", &[("GOLFER", &name)])`
fills its parameters, `lang::plural(n, one, other)` picks a key by a count, `lang::load_overlay(text)` lays a translation
(or a pack's) over the table, `lang::reset()` goes back to English. New text goes into `lang/en.txt` under a key named
`STR_<AREA>_<WHAT>` in the group of its area, with a comment saying where it shows and, for the original's text, the exe
address the code cites. Keys are always written out in full in the code (tables of keys are fine, keys built at run time
are not), so the tests can check that every key the code uses is in `lang/en.txt`, that none is there twice and that none
is unused (`cargo test -p sg-core lang`). A sample of the formatted messages is checked word for word against the text
the game made before its strings moved into the table (`lang_golden.rs`).
