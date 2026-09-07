"""
Script to generate an equipment.json of all the equipment on the OSRS Wiki, and downloads images for each item.
The JSON file is placed in ../src/databases/equipment.json.

The images are placed in ../src/images/equipment/.

Written for Python 3.9.
"""

import json
import os
import re
import urllib.parse

import requests

FILE_NAME = "../assets/json/equipment.json"
WIKI_BASE = "https://oldschool.runescape.wiki"
API_BASE = WIKI_BASE + "/api.php"
IMG_PATH = "../assets/equipment/"

BUCKET_API_FIELDS = [
    "page_name",
    "page_name_sub",
    "item_name",
    "image",
    "item_id",
    "weight",
    "version_anchor",
    "infobox_bonuses.crush_attack_bonus",
    "infobox_bonuses.crush_defence_bonus",
    "infobox_bonuses.equipment_slot",
    "infobox_bonuses.magic_damage_bonus",
    "infobox_bonuses.magic_attack_bonus",
    "infobox_bonuses.magic_defence_bonus",
    "infobox_bonuses.prayer_bonus",
    "infobox_bonuses.range_attack_bonus",
    "infobox_bonuses.ranged_strength_bonus",
    "infobox_bonuses.range_defence_bonus",
    "infobox_bonuses.slash_attack_bonus",
    "infobox_bonuses.slash_defence_bonus",
    "infobox_bonuses.stab_attack_bonus",
    "infobox_bonuses.stab_defence_bonus",
    "infobox_bonuses.strength_bonus",
    "infobox_bonuses.weapon_attack_range",
    "infobox_bonuses.weapon_attack_speed",
    "infobox_bonuses.combat_style",
    "infobox_bonuses.equipment_slot",
]

ITEMS_TO_SKIP = [
    "The dogsword",
    "Amulet of the monarchs",
    "Emperor ring",
    "Nature's reprisal",
    "Gloves of the damned",
    "Sunlight spear",
    "Sunlit bracers",
    "Thunder khopesh",
    "Thousand-dragon ward",
    "Arcane grimoire",
    "Wristbands of the arena",
    "Wristbands of the arena (i)",
    "Armadyl chainskirt (or)",
    "Armadyl chestplate (or)",
    "Armadyl helmet (or)",
    "Dagon'hai hat (or)",
    "Dagon'hai robe bottom (or)",
    "Dagon'hai robe top (or)",
    "Dragon warhammer (or)",
    "Centurion cuirass",
    "Ruinous powers (item)",
    "Battlehat",
    "Zaryte bow",
    "Corrupted armadyl godsword",
    "Corrupted dark bow",
    "Corrupted dragon claws",
    "Corrupted scythe of vitur",
    "Corrupted tumeken's shadow",
    "Corrupted twisted bow",
    "Corrupted voidwaker",
    "Corrupted volatile nightmare staff",
    "The dogsword (Deadman Mode)",
    "Thunder khopesh (Deadman Mode)",
]


def getEquipmentData():
    equipment = []
    offset = 0
    fields_csv = ",".join(map(repr, BUCKET_API_FIELDS))
    while True:
        print("Fetching equipment info: " + str(offset))
        query = {
            "action": "bucket",
            "format": "json",
            "query": (
                f"bucket('infobox_item')"
                f".select({fields_csv})"
                f".limit(500).offset({offset})"
                f".where('infobox_bonuses.equipment_slot', '!=', bucket.Null())"
                f".where('item_id', '!=', bucket.Null())"
                f".join('infobox_bonuses', 'infobox_bonuses.page_name_sub', 'infobox_item.page_name_sub')"
                f".orderBy('page_name_sub', 'asc').run()"
            ),
        }

        r = requests.get(
            API_BASE + "?" + urllib.parse.urlencode(query),
            headers={"User-Agent": "Orion (@jmyaeger on Discord)"},
        )

        data = r.json()

        if "bucket" not in data:
            # No results?
            break

        equipment = equipment + data["bucket"]

        # Bucket's API doesn't tell you when there are more results, so we'll just have to guess
        if len(data["bucket"]) == 500:
            offset += 500
        else:
            # If we are at the end of the results, break out of this loop
            break

    return equipment


def main():
    # Grab the equipment info using Bucket
    wiki_data = getEquipmentData()

    # Use an object rather than an array, so that we can't have duplicate items with the same page_name_sub
    data = {}
    required_imgs = []

    # Loop over the equipment data from the wiki
    for v in wiki_data:
        if v["page_name_sub"] in data:
            continue

        print(f"Processing {v['page_name_sub']}")

        try:
            item_id = int(v.get("item_id")[0]) if v.get("item_id") else None
        except ValueError:
            # Item has an invalid ID, do not show it here as it's probably historical or something.
            print("Skipping - invalid item ID (not an int)")
            continue

        attack_range = v.get("infobox_bonuses.weapon_attack_range")
        if attack_range is not None:
            attack_range = int(attack_range)

        equipment = {
            "name": v["page_name"],
            "id": item_id,
            "version": v.get("version_anchor"),
            "slot": v.get("infobox_bonuses.equipment_slot").lower(),
            "image": (
                "" if not v.get("image") else v.get("image")[-1].replace("File:", "")
            ),
            "speed": v.get("infobox_bonuses.weapon_attack_speed"),
            "category": v.get("infobox_bonuses.combat_style"),
            "bonuses": {
                "strength": {
                    "melee": v.get("infobox_bonuses.strength_bonus", 0),
                    "ranged": v.get("infobox_bonuses.ranged_strength_bonus", 0),
                    "magic": v.get("infobox_bonuses.magic_damage_bonus", 0.0),
                },
                "attack": {
                    "stab": v.get("infobox_bonuses.stab_attack_bonus", 0),
                    "slash": v.get("infobox_bonuses.slash_attack_bonus", 0),
                    "crush": v.get("infobox_bonuses.crush_attack_bonus", 0),
                    "magic": v.get("infobox_bonuses.magic_attack_bonus", 0),
                    "ranged": v.get("infobox_bonuses.range_attack_bonus", 0),
                },
                "defence": {
                    "stab": v.get("infobox_bonuses.stab_defence_bonus", 0),
                    "slash": v.get("infobox_bonuses.slash_defence_bonus", 0),
                    "crush": v.get("infobox_bonuses.crush_defence_bonus", 0),
                    "magic": v.get("infobox_bonuses.magic_defence_bonus", 0),
                    "ranged": v.get("infobox_bonuses.range_defence_bonus", 0),
                },
                "prayer": v.get("infobox_bonuses.prayer_bonus", 0),
            },
            "is_two_handed": None,
            "attack_range": attack_range,
        }

        # Handle 2H weapons
        if equipment["slot"] == "weapon":
            equipment["is_two_handed"] = False

        if equipment["slot"] == "2h":
            equipment["slot"] = "weapon"
            equipment["is_two_handed"] = True

        version = (
            str(equipment["version"]) if equipment["version"] is not None else None
        )

        if version is not None:
            # If this is an item from Nightmare Zone, it will become the main variant for all NMZ/SW/Emir's variants
            if version == "Nightmare Zone":
                equipment["version"] = None

            if re.match(r"^(Broken|0|25|50|75|100)$", version):
                continue

            if version in ["New", "Used"]:
                equipment["version"] = None

            if equipment["name"] == "Toxic blowpipe" and version in [
                "Empty",
                "Charged",
            ]:
                continue

            if (
                equipment["name"]
                in [
                    "Accursed sceptre",
                    "Accursed sceptre (a)",
                    "Corrupted tumeken's shadow",
                    "Craw's bow",
                    "Holy sanguinesti staff",
                    "Sanguinesti staff",
                    "Thammaron's sceptre",
                    "Thammaron's sceptre (a)",
                    "Trident of the seas",
                    "Trident of the seas (e)",
                    "Trident of the swamp",
                    "trident of the swamp (e)",
                    "Tumeken's shadow",
                    "Ursine chainmace",
                    "Viggora's chainmace",
                    "Warped sceptre",
                    "Webweaver bow",
                ]
                and version == "Uncharged"
            ):
                continue

            if (
                equipment["name"]
                in [
                    "Blade of saeldor",
                    "Bow of faerdhinen",
                    "Crystal body",
                    "Crystal helm",
                    "Crystal legs",
                    "Crystal shield",
                ]
                and version == "Inactive"
            ):
                continue

            if "Black mask" in equipment["name"]:
                if version in [
                    "1",
                    "2",
                    "3",
                    "4",
                    "5",
                    "6",
                    "7",
                    "8",
                    "9",
                    "10",
                ]:
                    continue

                if version == "Uncharged":
                    equipment["version"] = None

            if version in ["Locked", "Broken"]:
                continue

            if version in ["Normal", "Restored", "Undamaged"]:
                equipment["version"] = None

        # Skip last man standing items
        if "(Last Man Standing)" in equipment["name"]:
            continue

        if equipment["name"] in ITEMS_TO_SKIP:
            continue

        if "(unobtainable item)" in equipment["name"]:
            continue

        if (
            "Keris partisan of amascut" in equipment["name"]
            and "Outside ToA" in v["page_name_sub"]
        ):
            continue

        if "historical" in equipment["name"]:
            continue

        # Set the current equipment item to the calc's equipment list
        data[v["page_name_sub"]] = equipment

        if equipment["image"] != "":
            required_imgs.append(equipment["image"])

    new_data = list(data.values())

    # add manual equipment that isn't pulled from the wiki
    # this should ONLY be used for upcoming items that are not yet released
    with open("manual_equipment.json", "r") as f:
        manual_data = json.load(f)
        new_data = new_data + manual_data

    print("Total equipment: " + str(len(new_data)))
    new_data.sort(key=lambda d: d.get("name"))

    with open(FILE_NAME, "w") as f:
        print("Saving to JSON at file: " + FILE_NAME)
        json.dump(new_data, f, ensure_ascii=False, indent=2)

    success_img_dls = 0
    failed_img_dls = 0
    skipped_img_dls = 0
    required_imgs = set(required_imgs)

    # Fetch all the images from the wiki and store them for local serving
    for idx, img in enumerate(required_imgs):
        if os.path.isfile(IMG_PATH + img):
            skipped_img_dls += 1
            continue

        print(f"({idx}/{len(required_imgs)}) Fetching image: {img}")
        r = requests.get(
            WIKI_BASE + "/w/Special:Filepath/" + img,
            headers={"User-Agent": "Orion (@jmyaeger on Discord)"},
        )
        if r.ok:
            with open(IMG_PATH + img, "wb") as f:
                f.write(r.content)
                print("Saved image: " + img)
                success_img_dls += 1
        else:
            print("Unable to save image: " + img)
            failed_img_dls += 1

    print("Total images saved: " + str(success_img_dls))
    print("Total images skipped (already exists): " + str(skipped_img_dls))
    print("Total images failed to save: " + str(failed_img_dls))


main()
