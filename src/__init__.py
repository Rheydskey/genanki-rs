from dataclasses import dataclass
import os
from pathlib import Path
from anki.collection import Collection
from anki.decks import DeckId
from aqt import mw
from aqt import gui_hooks
from aqt.operations import QueryOp
from .gencore import from_config

BASE_PATH = Path(__file__).parent / "user_files"

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
class DeckOutput:
    added: list[Card]
    deleted: list[str]

    @staticmethod
    def from_dict(dict_data: dict[str, list[dict[str, str] | str]]):
        return DeckOutput(Card.from_list(dict_data["added"]), dict_data["deleted"])


@dataclass
class Output:
    decks: dict[str, DeckOutput]

    @staticmethod
    def from_dict(output: dict[str, dict[str, list[Card | str]]]):
        return Output({k: DeckOutput.from_dict(v) for (k, v) in output.items()})


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


def update_from_config() -> Output:
    config_path = "./config.toml"
    value = from_config(config_path)
    return Output.from_dict(value)


class Config:
    def __init__(self, url: str, col: Collection) -> None:
        self.url: str = url
        self.collection: Collection = col

    def execute(self) -> int:
        decks = update_from_config()
        if "Ankill" not in [n.name for n in self.collection.models.all_names_and_ids()]:
            self.collection.models.save(create_model())

        for name, diff in decks.decks.items():
            deckid = create_or_get_deck_for_name(self.collection, name)
            delete_cards(self.collection, deckid, diff.deleted)
            add_cards(
                self.collection,
                deckid,
                diff.added,
            )

        return 0


def init() -> None:
    os.chdir(BASE_PATH)
    mw.create_backup_now()
    op = QueryOp(
        parent=mw,
        op=lambda col: Config("./config.toml", col).execute(),
        success=lambda e: None,
    )
    op.with_progress(label="Updating your decks...").run_in_background()
    mw.deckBrowser.refresh()


gui_hooks.profile_did_open.append(init)
