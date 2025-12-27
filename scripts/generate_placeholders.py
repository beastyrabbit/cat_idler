import os
from PIL import Image, ImageDraw, ImageFont

def create_placeholder(path, size, color, text, shape="rect"):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    img = Image.new('RGBA', size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # Draw shape
    if shape == "rect":
        draw.rectangle([(0, 0), (size[0]-1, size[1]-1)], fill=color, outline="black")
    elif shape == "circle":
        draw.ellipse([(0, 0), (size[0]-1, size[1]-1)], fill=color, outline="black")
    
    # Draw text
    try:
        # Try to load a default font, otherwise use default
        font = ImageFont.load_default()
    except:
        font = None
        
    # Simple text centering
    # text_width = draw.textlength(text, font=font) # newer PIL
    # For older PIL compatibility we might just draw at top left or center roughly
    draw.text((5, size[1]/2 - 5), text, fill="black", font=font)
    
    img.save(path)
    print(f"Created {path}")

# 1. Cat Sprites
# Variants: Orange Tabby, Gray Tabby, Black, White, Calico, Tuxedo
# Poses: idle, walking, sleeping, eating, alert
cat_colors = {
    "orange-tabby": "#FFA500",
    "gray-tabby": "#808080",
    "black": "#303030",
    "white": "#F0F0F0",
    "calico": "#D2691E",
    "tuxedo": "#000000"
}

cat_poses = ["idle", "walking", "sleeping", "eating", "alert"]

for variant, color in cat_colors.items():
    for pose in cat_poses:
        create_placeholder(
            f"public/images/cats/{variant}/{pose}.png",
            (128, 128),
            color,
            f"{variant}\n{pose}",
            shape="circle"
        )

# 2. Buildings
buildings = {
    "den": "#8B4513",
    "food_storage": "#CD853F",
    "water_bowl": "#4682B4",
    "beds": "#F5DEB3",
    "herb_garden": "#228B22",
    "nursery": "#FFB6C1",
    "elder_corner": "#DEB887",
    "walls": "#696969",
    "mouse_farm": "#A0522D"
}

for name, color in buildings.items():
    create_placeholder(
        f"public/images/buildings/{name}.png",
        (150, 150),
        color,
        name
    )
    # Construction states
    create_placeholder(
        f"public/images/buildings/{name}-blueprint.png",
        (150, 150),
        "#A9A9A9",
        f"{name}\nblueprint"
    )

# 3. Tiles
tiles = {
    "field": "#9ACD32",
    "forest": "#006400",
    "dense_woods": "#004d00",
    "river": "#1E90FF",
    "enemy_territory": "#8B0000"
}

for name, color in tiles.items():
    create_placeholder(
        f"public/images/tiles/{name}.png",
        (150, 150),
        color,
        name
    )

# 4. Enemies
enemies = {
    "fox": "#FF4500",
    "hawk": "#8B4513",
    "badger": "#808080",
    "bear": "#4A2F1B",
    "rival_cat": "#696969"
}

sizes = {
    "fox": (192, 192),
    "hawk": (192, 192),
    "badger": (192, 192),
    "bear": (256, 256),
    "rival_cat": (128, 128)
}

for name, color in enemies.items():
    create_placeholder(
        f"public/images/enemies/{name}.png",
        sizes.get(name, (128, 128)),
        color,
        name,
        shape="circle"
    )

# 5. Resources
resources = {
    "food": "#CD5C5C",
    "water": "#00BFFF",
    "herbs": "#32CD32",
    "materials": "#8B4513",
    "blessings": "#FFD700"
}

for name, color in resources.items():
    create_placeholder(
        f"public/images/resources/{name}.png",
        (48, 48),
        color,
        name[0].upper()
    )


# 6. Task Icons
tasks = {
    "hunt": "#8B4513",
    "gather_herbs": "#228B22",
    "fetch_water": "#1E90FF",
    "clean": "#D3D3D3",
    "build": "#DEB887",
    "guard": "#696969",
    "heal": "#FF69B4",
    "kitsit": "#FFB6C1",
    "explore": "#DAA520",
    "patrol": "#708090",
    "teach": "#800080",
    "rest": "#ADD8E6"
}

for name, color in tasks.items():
    create_placeholder(
        f"public/images/ui/tasks/{name}.png",
        (64, 64),
        color,
        name[0].upper(),
        shape="circle"
    )

# 7. Status Icons
statuses = {
    "leader": "#FFD700",
    "pregnant": "#FFC0CB",
    "injured": "#FF0000",
    "hungry": "#D2691E",
    "thirsty": "#00BFFF",
    "tired": "#808080",
    "happy": "#FF1493"
}

for name, color in statuses.items():
    create_placeholder(
        f"public/images/ui/status/{name}.png",
        (32, 32),
        color,
        name[0].upper(),
        shape="circle"
    )

print("All placeholders created!")

