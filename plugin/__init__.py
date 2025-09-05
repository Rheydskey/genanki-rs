from dataclasses import dataclass
import hashlib
import json
import subprocess
import sys
import os
from typing import Any
from pathlib import Path
from anki.collection import Collection
from anki.decks import DeckId
from aqt import mw

from aqt import gui_hooks
from aqt.operations import QueryOp

sys.path.insert(0, str(Path(os.path.dirname(__file__)) / "libs"))

BASE_PATH = Path(__file__).parent

static_html = """
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css" integrity="sha384-nB0miv6/jRmo5UMMR1wu3Gz6NLsoTkbqJghGIsx//Rlm+ZU03BU6SQNC66uf4l5+" crossorigin="anonymous">
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js" integrity="sha384-7zkQWkzuo3B5mTepMUcHkMB5jZaolc2xDwL6VFqjFALcbeS9Ggm/Yr2r3Dy4lfFg" crossorigin="anonymous"></script>
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/contrib/auto-render.min.js" integrity="sha384-43gviWU0YVjaDtb/GhzOouOXtZMP/7XUzwPTstBeZFe/+rCMvRwr4yROQP43s0Xk" crossorigin="anonymous" onload="renderMathInElement(document.body);"></script>
"""

col = mw.col
ext_pwd = Path(os.path.dirname(__file__))
card_folder = ext_pwd / "cards/"
git_repo = "https://git.rheydskey.org/rheydskey/anki-md"


def folder_name(url: str) -> str:
    return hashlib.sha256(url.encode()).hexdigest()[0:6]


@dataclass
class Card:
    front: str
    back: str
    hash: str

    @staticmethod
    def fromJson(json: dict[str, str]) -> "Card":
        return Card(json["front"], json["back"], json["hash"])


@dataclass
class InitOutput:
    decks: dict[str, list[Card]]


@dataclass
class DiffOutput:
    added: list[Card]
    deleted: list[str]


@dataclass
class UpdateOutput:
    decks: dict[str, list[DiffOutput]]


class Gencore:
    def __init__(self, path: Path | None = None):
        self.path: Path = Path(BASE_PATH) / "gencore" if path is None else path

    def call(self, args: list[str]) -> str:
        process = subprocess.run([str(self.path)] + args, capture_output=True)
        print(process.stderr.decode())
        process.check_returncode()
        return process.stdout.decode()

    def init(self, url: str) -> str:
        output_folder = folder_name(url)
        return self.call(["init", url, output_folder])

    def update(self, folder: str) -> str:
        return self.call(["update", folder])


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
        note = col.new_note(model)
        note.fields[0] = card.front
        note.fields[1] = card.back
        note.fields[2] = card.hash
        _ = col.add_note(note, deck_id)


def delete_cards(col: Collection, did: DeckId, hashes: list[str]):
    for hash in hashes:
        query = f"did:{did} hash:{hash}"
        print(f"deleted in ({did})", list(col.find_cards(query)))
        col.remove_notes_by_card(list(col.find_cards(query)))


def create_or_get_deck_for_name(col: Collection, deck_name: str) -> DeckId:
    deckid = col.decks.id_for_name(deck_name)
    if deckid is None:
        deck = col.decks.new_deck()
        deck.name = deck_name
        id = col.decks.add_deck(deck)
        return DeckId(id.id)

    return deckid


def manage_card(col: Collection) -> int:
    gencore = Gencore()

    if "Ankill" not in [n.name for n in col.models.all_names_and_ids()]:
        col.models.save(create_model())

    if not Path(folder_name(git_repo)).exists():
        i = gencore.init(git_repo)
        l = json.loads(i)
        decks: dict[str, list[object]] = l["decks"].items()
        for name, cards in decks:
            deckid = create_or_get_deck_for_name(col, name)
            add_cards(
                col,
                deckid,
                list(map(Card.fromJson, cards)),
            )

    else:
        input = gencore.update(folder_name(git_repo))
        if input == "":
            return 0

        decks: dict[str, Any] = dict(json.loads(input)["decks"])
        for name, diff in decks.items():
            deckid = create_or_get_deck_for_name(col, name)
            delete_cards(col, deckid, diff["deleted"])
            add_cards(
                col,
                deckid,
                list(map(Card.fromJson, diff["added"])),
            )

    return 0


def init() -> None:
    os.chdir(ext_pwd)

    mw.create_backup_now()

    op = QueryOp(
        parent=mw,
        op=manage_card,
        success=lambda e: None,
    )

    op.with_progress(label="Update git repo").run_in_background()
    mw.deckBrowser.refresh()


gui_hooks.profile_did_open.append(init)
