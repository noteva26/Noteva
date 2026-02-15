/**
 * Extended emoji data for Vditor editor panel.
 * Keys are shortcodes, values are Unicode emoji characters.
 * Grouped by category for maintainability.
 */

const smileys: Record<string, string> = {
  "grinning": "😀", "smiley": "😃", "smile": "😄", "grin": "😁", "laughing": "😆",
  "sweat_smile": "😅", "rofl": "🤣", "joy": "😂", "slightly_smiling_face": "🙂",
  "upside_down_face": "🙃", "melting_face": "🫠", "wink": "😉", "blush": "😊",
  "innocent": "😇", "smiling_face_with_three_hearts": "🥰", "heart_eyes": "😍",
  "star_struck": "🤩", "kissing_heart": "😘", "kissing": "😗",
  "kissing_closed_eyes": "😚", "kissing_smiling_eyes": "😙", "smiling_face_with_tear": "🥲",
  "yum": "😋", "stuck_out_tongue": "😛", "stuck_out_tongue_winking_eye": "😜",
  "zany_face": "🤪", "stuck_out_tongue_closed_eyes": "😝", "money_mouth_face": "🤑",
  "hugs": "🤗", "hand_over_mouth": "🤭", "shushing_face": "🤫", "thinking": "🤔",
  "saluting_face": "🫡", "zipper_mouth_face": "🤐", "raised_eyebrow": "🤨",
  "neutral_face": "😐", "expressionless": "😑", "no_mouth": "😶",
  "dotted_line_face": "🫥", "face_in_clouds": "😶‍🌫️", "smirk": "😏",
  "unamused": "😒", "roll_eyes": "🙄", "grimacing": "😬", "face_exhaling": "😮‍💨",
  "lying_face": "🤥", "shaking_face": "🫨", "relieved": "😌", "pensive": "😔",
  "sleepy": "😪", "drooling_face": "🤤", "sleeping": "😴", "mask": "😷",
  "face_with_thermometer": "🤒", "face_with_head_bandage": "🤕", "nauseated_face": "🤢",
  "vomiting": "🤮", "sneezing_face": "🤧", "hot": "🥵", "cold": "🥶",
  "woozy_face": "🥴", "dizzy_face": "😵", "face_with_spiral_eyes": "😵‍💫",
  "exploding_head": "🤯", "cowboy_hat_face": "🤠", "partying_face": "🥳",
  "disguised_face": "🥸", "sunglasses": "😎", "nerd_face": "🤓", "monocle_face": "🧐",
  "confused": "😕", "worried": "😟", "slightly_frowning_face": "🙁", "frowning_face": "☹️",
  "open_mouth": "😮", "hushed": "😯", "astonished": "😲", "flushed": "😳",
  "pleading_face": "🥺", "face_holding_back_tears": "🥹", "frowning": "😦",
  "anguished": "😧", "fearful": "😨", "cold_sweat": "😰", "disappointed_relieved": "😥",
  "cry": "😢", "sob": "😭", "scream": "😱", "confounded": "😖", "persevere": "😣",
  "disappointed": "😞", "sweat": "😓", "weary": "😩", "tired_face": "😫",
  "yawning_face": "🥱", "triumph": "😤", "rage": "😡", "angry": "😠",
  "cursing_face": "🤬", "smiling_imp": "😈", "imp": "👿", "skull": "💀",
  "skull_and_crossbones": "☠️", "poop": "💩", "clown_face": "🤡", "japanese_ogre": "👹",
  "japanese_goblin": "👺", "ghost": "👻", "alien": "👽", "space_invader": "👾",
  "robot": "🤖",
};

const gestures: Record<string, string> = {
  "wave": "👋", "raised_back_of_hand": "🤚", "hand": "✋", "vulcan_salute": "🖖",
  "rightwards_hand": "🫱", "leftwards_hand": "🫲", "palm_down_hand": "🫳",
  "palm_up_hand": "🫴", "ok_hand": "👌", "pinched_fingers": "🤌", "pinching_hand": "🤏",
  "v": "✌️", "crossed_fingers": "🤞", "hand_with_index_finger_and_thumb_crossed": "🫰",
  "love_you_gesture": "🤟", "metal": "🤘", "call_me_hand": "🤙",
  "point_left": "👈", "point_right": "👉", "point_up_2": "👆", "middle_finger": "🖕",
  "point_down": "👇", "point_up": "☝️", "index_pointing_at_the_viewer": "🫵",
  "+1": "👍", "-1": "👎", "fist": "✊", "facepunch": "👊", "left_facing_fist": "🤛",
  "right_facing_fist": "🤜", "clap": "👏", "raised_hands": "🙌", "heart_hands": "🫶",
  "open_hands": "👐", "palms_up_together": "🤲", "handshake": "🤝", "pray": "🙏",
  "writing_hand": "✍️", "nail_care": "💅", "selfie": "🤳", "muscle": "💪",
};

const hearts: Record<string, string> = {
  "heart": "❤️", "orange_heart": "🧡", "yellow_heart": "💛", "green_heart": "💚",
  "blue_heart": "💙", "purple_heart": "💜", "black_heart": "🖤", "white_heart": "🤍",
  "brown_heart": "🤎", "pink_heart": "🩷", "light_blue_heart": "🩵", "grey_heart": "🩶",
  "broken_heart": "💔", "heart_exclamation": "❣️", "two_hearts": "💕",
  "revolving_hearts": "💞", "heartbeat": "💓", "heartpulse": "💗",
  "growing_heart": "💖", "sparkling_heart": "💖", "cupid": "💘",
  "gift_heart": "💝", "heart_decoration": "💟", "heart_on_fire": "❤️‍🔥",
  "mending_heart": "❤️‍🩹", "love_letter": "💌", "kiss": "💋",
  "100": "💯", "anger": "💢", "boom": "💥", "dizzy": "💫",
  "sweat_drops": "💦", "dash": "💨", "hole": "🕳️", "speech_balloon": "💬",
  "thought_balloon": "💭", "zzz": "💤",
};

const animals: Record<string, string> = {
  "monkey_face": "🐵", "monkey": "🐒", "gorilla": "🦍", "orangutan": "🦧",
  "dog": "🐶", "dog2": "🐕", "guide_dog": "🦮", "poodle": "🐩", "wolf": "🐺",
  "fox_face": "🦊", "raccoon": "🦝", "cat": "🐱", "cat2": "🐈", "black_cat": "🐈‍⬛",
  "lion": "🦁", "tiger": "🐯", "tiger2": "🐅", "leopard": "🐆",
  "horse": "🐴", "unicorn": "🦄", "zebra": "🦓", "deer": "🦌",
  "bison": "🦬", "cow": "🐮", "ox": "🐂", "water_buffalo": "🐃",
  "pig": "🐷", "pig2": "🐖", "boar": "🐗", "pig_nose": "🐽",
  "ram": "🐏", "sheep": "🐑", "goat": "🐐", "camel": "🐪",
  "llama": "🦙", "giraffe": "🦒", "elephant": "🐘", "mammoth": "🦣",
  "rhinoceros": "🦏", "hippopotamus": "🦛",
  "mouse": "🐭", "mouse2": "🐁", "rat": "🐀", "hamster": "🐹",
  "rabbit": "🐰", "rabbit2": "🐇", "chipmunk": "🐿️", "beaver": "🦫",
  "hedgehog": "🦔", "bat": "🦇", "bear": "🐻", "polar_bear": "🐻‍❄️",
  "koala": "🐨", "panda_face": "🐼", "sloth": "🦥", "otter": "🦦",
  "skunk": "🦨", "kangaroo": "🦘", "badger": "🦡",
  "turkey": "🦃", "chicken": "🐔", "rooster": "🐓", "hatching_chick": "🐣",
  "baby_chick": "🐤", "hatched_chick": "🐥", "bird": "🐦", "penguin": "🐧",
  "dove": "🕊️", "eagle": "🦅", "duck": "🦆", "swan": "🦢", "owl": "🦉",
  "dodo": "🦤", "feather": "🪶", "flamingo": "🦩", "peacock": "🦚", "parrot": "🦜",
  "frog": "🐸", "crocodile": "🐊", "turtle": "🐢", "lizard": "🦎",
  "snake": "🐍", "dragon_face": "🐲", "dragon": "🐉", "sauropod": "🦕", "t_rex": "🦖",
  "whale": "🐳", "whale2": "🐋", "dolphin": "🐬", "seal": "🦭",
  "fish": "🐟", "tropical_fish": "🐠", "blowfish": "🐡", "shark": "🦈",
  "octopus": "🐙", "shell": "🐚", "coral": "🪸", "jellyfish": "🪼",
  "snail": "🐌", "butterfly": "🦋", "bug": "🐛", "ant": "🐜", "bee": "🐝",
  "beetle": "🪲", "ladybug": "🐞", "cricket": "🦗", "cockroach": "🪳",
  "spider": "🕷️", "spider_web": "🕸️", "scorpion": "🦂",
};

const food: Record<string, string> = {
  "apple": "🍎", "green_apple": "🍏", "pear": "🍐", "tangerine": "🍊",
  "lemon": "🍋", "banana": "🍌", "watermelon": "🍉", "grapes": "🍇",
  "strawberry": "🍓", "melon": "🍈", "cherries": "🍒", "peach": "🍑",
  "mango": "🥭", "pineapple": "🍍", "coconut": "🥥", "kiwi_fruit": "🥝",
  "tomato": "🍅", "eggplant": "🍆", "avocado": "🥑", "broccoli": "🥦",
  "carrot": "🥕", "corn": "🌽", "hot_pepper": "🌶️", "cucumber": "🥒",
  "mushroom": "🍄", "peanuts": "🥜", "chestnut": "🌰",
  "bread": "🍞", "croissant": "🥐", "baguette_bread": "🥖", "pretzel": "🥨",
  "bagel": "🥯", "pancakes": "🥞", "waffle": "🧇", "cheese": "🧀",
  "meat_on_bone": "🍖", "poultry_leg": "🍗", "bacon": "🥓",
  "hamburger": "🍔", "fries": "🍟", "pizza": "🍕", "hotdog": "🌭",
  "sandwich": "🥪", "taco": "🌮", "burrito": "🌯", "tamale": "🫔",
  "egg": "🥚", "cooking": "🍳",
  "rice": "🍚", "curry": "🍛", "ramen": "🍜", "spaghetti": "🍝",
  "sushi": "🍣", "bento": "🍱", "dumpling": "🥟",
  "ice_cream": "🍨", "shaved_ice": "🍧", "icecream": "🍦", "doughnut": "🍩",
  "cookie": "🍪", "birthday": "🎂", "cake": "🍰", "cupcake": "🧁",
  "pie": "🥧", "chocolate_bar": "🍫", "candy": "🍬", "lollipop": "🍭",
  "coffee": "☕", "tea": "🍵", "bubble_tea": "🧋", "sake": "🍶",
  "beer": "🍺", "beers": "🍻", "wine_glass": "🍷", "cocktail": "🍸",
  "tropical_drink": "🍹", "champagne": "🍾",
};

const travel: Record<string, string> = {
  "car": "🚗", "taxi": "🚕", "bus": "🚌", "trolleybus": "🚎",
  "racing_car": "🏎️", "police_car": "🚓", "ambulance": "🚑", "fire_engine": "🚒",
  "minibus": "🚐", "truck": "🚚", "articulated_lorry": "🚛",
  "tractor": "🚜", "motorcycle": "🏍️", "bicycle": "🚲", "scooter": "🛴",
  "airplane": "✈️", "rocket": "🚀", "flying_saucer": "🛸",
  "ship": "🚢", "sailboat": "⛵", "speedboat": "🚤",
  "train": "🚋", "metro": "🚇", "light_rail": "🚈", "station": "🚉",
  "helicopter": "🚁", "canoe": "🛶",
  "house": "🏠", "office": "🏢", "hospital": "🏥", "school": "🏫",
  "church": "⛪", "mosque": "🕌", "temple": "🛕",
  "sunrise": "🌅", "sunset": "🌇", "night_with_stars": "🌃",
  "camping": "🏕️", "beach_umbrella": "🏖️", "desert": "🏜️",
  "mountain": "⛰️", "volcano": "🌋", "mount_fuji": "🗻",
  "world_map": "🗺️", "compass": "🧭",
};

const objects: Record<string, string> = {
  "watch": "⌚", "iphone": "📱", "computer": "💻", "keyboard": "⌨️",
  "desktop_computer": "🖥️", "printer": "🖨️", "mouse_computer": "🖱️",
  "cd": "💿", "dvd": "📀", "floppy_disk": "💾",
  "camera": "📷", "video_camera": "📹", "movie_camera": "🎥", "tv": "📺",
  "radio": "📻", "telephone": "☎️", "bulb": "💡", "flashlight": "🔦",
  "candle": "🕯️", "fire": "🔥", "bomb": "💣",
  "gem": "💎", "money_with_wings": "💸", "dollar": "💵", "credit_card": "💳",
  "envelope": "✉️", "email": "📧", "package": "📦",
  "pencil2": "✏️", "pen": "🖊️", "paintbrush": "🖌️", "crayon": "🖍️",
  "memo": "📝", "briefcase": "💼", "file_folder": "📁",
  "clipboard": "📋", "calendar": "📅", "pushpin": "📌", "paperclip": "📎",
  "scissors": "✂️", "lock": "🔒", "unlock": "🔓", "key": "🔑",
  "hammer": "🔨", "wrench": "🔧", "gear": "⚙️", "link": "🔗",
  "mag": "🔍", "mag_right": "🔎",
};

const symbols: Record<string, string> = {
  "warning": "⚠️", "no_entry": "⛔", "prohibited": "🚫", "x": "❌",
  "o": "⭕", "bangbang": "‼️", "question": "❓", "exclamation": "❗",
  "checkmark": "✅", "white_check_mark": "✅", "ballot_box_with_check": "☑️",
  "heavy_check_mark": "✔️", "heavy_multiplication_x": "✖️",
  "star": "⭐", "star2": "🌟", "sparkles": "✨", "zap": "⚡",
  "sunny": "☀️", "cloud": "☁️", "umbrella": "☂️", "snowflake": "❄️",
  "rainbow": "🌈", "ocean": "🌊",
  "recycle": "♻️", "trident": "🔱", "fleur_de_lis": "⚜️",
  "beginner": "🔰", "heavy_dollar_sign": "💲",
  "arrow_up": "⬆️", "arrow_down": "⬇️", "arrow_left": "⬅️", "arrow_right": "➡️",
  "arrows_counterclockwise": "🔄", "back": "🔙", "end": "🔚",
  "new": "🆕", "free": "🆓", "up": "🆙", "cool": "🆒", "ok": "🆗",
  "sos": "🆘", "no_entry_sign": "🚫",
  "1234": "🔢", "hash": "#️⃣", "keycap_star": "*️⃣",
  "zero": "0️⃣", "one": "1️⃣", "two": "2️⃣", "three": "3️⃣", "four": "4️⃣",
  "five": "5️⃣", "six": "6️⃣", "seven": "7️⃣", "eight": "8️⃣", "nine": "9️⃣", "ten": "🔟",
};

const activities: Record<string, string> = {
  "soccer": "⚽", "basketball": "🏀", "football": "🏈", "baseball": "⚾",
  "tennis": "🎾", "volleyball": "🏐", "rugby_football": "🏉",
  "8ball": "🎱", "ping_pong": "🏓", "badminton": "🏸",
  "goal_net": "🥅", "ice_hockey": "🏒", "field_hockey": "🏑",
  "cricket_game": "🏏", "golf": "⛳", "bow_and_arrow": "🏹",
  "fishing_pole_and_fish": "🎣", "boxing_glove": "🥊", "martial_arts_uniform": "🥋",
  "ice_skate": "⛸️", "ski": "🎿", "sled": "🛷",
  "trophy": "🏆", "medal_sports": "🏅", "medal_military": "🎖️",
  "1st_place_medal": "🥇", "2nd_place_medal": "🥈", "3rd_place_medal": "🥉",
  "dart": "🎯", "kite": "🪁", "yo_yo": "🪀", "video_game": "🎮",
  "joystick": "🕹️", "jigsaw": "🧩", "teddy_bear": "🧸",
  "chess_pawn": "♟️", "performing_arts": "🎭", "art": "🎨",
  "musical_note": "🎵", "notes": "🎶", "microphone": "🎤",
  "headphones": "🎧", "saxophone": "🎷", "guitar": "🎸",
  "piano": "🎹", "trumpet": "🎺", "violin": "🎻", "drum": "🥁",
  "clapper": "🎬", "tada": "🎉", "confetti_ball": "🎊",
  "balloon": "🎈", "gift": "🎁", "ribbon": "🎀",
  "christmas_tree": "🎄", "jack_o_lantern": "🎃", "firecracker": "🧨",
};

const flags: Record<string, string> = {
  "checkered_flag": "🏁", "triangular_flag_on_post": "🚩", "crossed_flags": "🎌",
  "black_flag": "🏴", "white_flag": "🏳️", "rainbow_flag": "🏳️‍🌈",
  "pirate_flag": "🏴‍☠️",
  "cn": "🇨🇳", "us": "🇺🇸", "jp": "🇯🇵", "kr": "🇰🇷", "gb": "🇬🇧",
  "de": "🇩🇪", "fr": "🇫🇷", "es": "🇪🇸", "it": "🇮🇹", "ru": "🇷🇺",
  "br": "🇧🇷", "ca": "🇨🇦", "au": "🇦🇺", "in": "🇮🇳",
  "hk": "🇭🇰", "tw": "🇹🇼", "sg": "🇸🇬", "my": "🇲🇾",
};

/** Category definitions for emoji picker UI */
export interface EmojiCategory {
  id: string;
  label: Record<string, string>; // i18n labels keyed by locale
  icon: string; // Unicode emoji as category icon
  emojis: Record<string, string>;
}

export const EMOJI_CATEGORIES: EmojiCategory[] = [
  { id: "smileys", label: { "zh-CN": "表情", "zh-TW": "表情", "en": "Smileys" }, icon: "😀", emojis: smileys },
  { id: "gestures", label: { "zh-CN": "手势", "zh-TW": "手勢", "en": "Gestures" }, icon: "👋", emojis: gestures },
  { id: "hearts", label: { "zh-CN": "心形", "zh-TW": "心形", "en": "Hearts" }, icon: "❤️", emojis: hearts },
  { id: "animals", label: { "zh-CN": "动物", "zh-TW": "動物", "en": "Animals" }, icon: "🐱", emojis: animals },
  { id: "food", label: { "zh-CN": "食物", "zh-TW": "食物", "en": "Food" }, icon: "🍔", emojis: food },
  { id: "travel", label: { "zh-CN": "旅行", "zh-TW": "旅行", "en": "Travel" }, icon: "🚗", emojis: travel },
  { id: "objects", label: { "zh-CN": "物品", "zh-TW": "物品", "en": "Objects" }, icon: "💻", emojis: objects },
  { id: "symbols", label: { "zh-CN": "符号", "zh-TW": "符號", "en": "Symbols" }, icon: "⭐", emojis: symbols },
  { id: "activities", label: { "zh-CN": "活动", "zh-TW": "活動", "en": "Activities" }, icon: "⚽", emojis: activities },
  { id: "flags", label: { "zh-CN": "旗帜", "zh-TW": "旗幟", "en": "Flags" }, icon: "🏁", emojis: flags },
];

/** Complete emoji map for Vditor hint.emoji */
export const EMOJI_MAP: Record<string, string> = {
  ...smileys,
  ...gestures,
  ...hearts,
  ...animals,
  ...food,
  ...travel,
  ...objects,
  ...symbols,
  ...activities,
  ...flags,
};
