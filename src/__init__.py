from dataclasses import dataclass
import hashlib
import os
from pathlib import Path
from anki.collection import Collection
from anki.decks import DeckId
from aqt import mw
from aqt import gui_hooks
from aqt.operations import QueryOp
from .gencore import init as gencore_init, update as gencore_update


BASE_PATH = Path(__file__).parent
static_html = """
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css" integrity="sha384-nB0miv6/jRmo5UMMR1wu3Gz6NLsoTkbqJghGIsx//Rlm+ZU03BU6SQNC66uf4l5+" crossorigin="anonymous">
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js" integrity="sha384-7zkQWkzuo3B5mTepMUcHkMB5jZaolc2xDwL6VFqjFALcbeS9Ggm/Yr2r3Dy4lfFg" crossorigin="anonymous"></script>
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/contrib/auto-render.min.js" integrity="sha384-43gviWU0YVjaDtb/GhzOouOXtZMP/7XUzwPTstBeZFe/+rCMvRwr4yROQP43s0Xk" crossorigin="anonymous" onload="renderMathInElement(document.body);"></script>
<script>
    renderMathInElement(document.body, {
      // customised options
      // • auto-render specific keys, e.g.:
      delimiters: [
          {left: '$$', right: '$$', display: true},
          {left: '$', right: '$', display: false},
      ],
      // • rendering keys, e.g.:
      throwOnError : false
    });
</script>
"""

ext_pwd = Path(os.path.dirname(__file__))
git_repo: str = "https://git.rheydskey.org/rheydskey/l3-anki-md.git"


def folder_name(url: str) -> str:
    return hashlib.sha256(url.encode()).hexdigest()[0:6]


@dataclass
class Card:
    front: str
    back: str
    hash: str

    @staticmethod
    def from_dict(dict_data: dict[str, str]) -> "Card":
        return Card(dict_data["front"], dict_data["back"], dict_data["hash"])

    @staticmethod
    def from_list(list: list[dict[str, str]]) -> list["Card"]:
        return [Card.from_dict(d) for d in list]

    def exists_in(self, did: DeckId, col: Collection) -> bool:
        query = f"hash:{self.hash} did:{did} "
        return len(col.find_cards(query)) != 0


@dataclass
class InitOutput:
    decks: dict[str, list[Card]]

    @staticmethod
    def from_dict(dict_data: dict[str, dict[str, str]]):
        print(dict_data)
        return InitOutput({k: Card.from_list(v) for (k, v) in dict_data.items()})


@dataclass
class DiffOutput:
    added: list[Card]
    deleted: list[str]

    @staticmethod
    def from_dict(dict_data: dict[str, list[dict[str, str] | str]]):
        return DiffOutput(Card.from_list(dict["added"]), dict_data["deleted"])


@dataclass
class UpdateOutput:
    decks: dict[str, DiffOutput]

    @staticmethod
    def from_dict(output: dict[str, dict[str, list[Card | str]]]):
        return UpdateOutput({k: DiffOutput.from_dict(v) for (k, v) in output.items()})


def create_model():
    col = mw.col
    if col is None:
        raise Exception("Error")

    model = col.models.new("Ankill")
    recto = col.models.new_field("Recto")
    col.models.add_field(model, recto)
    verso = col.models.new_field("Verso")
    col.models.add_field(model, verso)
    hash = col.models.new_field("Hash")
    hash["collapsed"] = True
    col.models.add_field(model, hash)
    template = col.models.new_template("Carte")
    template["qfmt"] = "{{Recto}}" + static_html
    template["afmt"] = "{{FrontSide}}\n\n<hr id=answer>\n\n{{Verso}}"
    col.models.add_template(model, template)
    return model


def add_cards(col: Collection, deck_id: DeckId, cards: list[Card]):
    model = col.models.by_name("Ankill")
    if model is None:
        return

    for card in cards:
        if card.exists_in(deck_id, col):
            continue

        note = col.new_note(model)
        note.fields[0] = card.front
        note.fields[1] = card.back
        note.fields[2] = card.hash
        _ = col.add_note(note, deck_id)


def delete_cards(col: Collection, did: DeckId, hashes: list[str]):
    for hash in hashes:
        query = f"did:{did} hash:{hash}"
        col.remove_notes_by_card(list(col.find_cards(query)))


def create_or_get_deck_for_name(col: Collection, deck_name: str) -> DeckId:
    deckid = col.decks.id_for_name(deck_name)
    if deckid is None:
        deck = col.decks.new_deck()
        deck.name = deck_name
        id = col.decks.add_deck(deck)
        return DeckId(id.id)

    return deckid


class Repository:
    def __init__(self, url: str, col: Collection):
        self.url: str = url
        self.collection: Collection = col

    def manage(self) -> int:
        if "Ankill" not in [n.name for n in self.collection.models.all_names_and_ids()]:
            self.collection.models.save(create_model())

        if not Path(folder_name(git_repo)).exists():
            self._create()
        else:
            self._update()

        return 0

    def _create(self) -> None:
        decks = InitOutput.from_dict(gencore_init(self.url, folder_name(git_repo)))
        for name, cards in decks.decks.items():
            deckid = create_or_get_deck_for_name(self.collection, name)
            add_cards(
                self.collection,
                deckid,
                cards,
            )

    def _update(self) -> None:
        decks = UpdateOutput.from_dict(gencore_update(folder_name(git_repo)))
        for name, diff in decks.decks.items():
            deckid = create_or_get_deck_for_name(self.collection, name)
            delete_cards(self.collection, deckid, diff.deleted)
            add_cards(
                self.collection,
                deckid,
                diff.added,
            )


def init() -> None:
    os.chdir(ext_pwd)

    mw.create_backup_now()

    op = QueryOp(
        parent=mw,
        op=lambda col: Repository(git_repo, col).manage(),
        success=lambda e: None,
    )

    op.with_progress(label="Update git repo").run_in_background()
    mw.deckBrowser.refresh()


gui_hooks.profile_did_open.append(init)
