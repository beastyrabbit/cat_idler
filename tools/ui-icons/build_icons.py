#!/usr/bin/env python3
"""Build the HUD icon and font assets under unity/Assets/Resources/UI from tracked sources.

* Line glyphs: every entry of docs/ui-redesign/mockups/icons.js (24 px viewBox, stroke 1.8, round caps
  and joins) becomes Resources/UI/Icons/<name>.svg drawn in white, imported by Unity as a UI Toolkit
  VectorImage (svgType 3) and tinted by USS.
* Resource silhouettes: public/images/game/icons/*.png are copied to Resources/UI/Icons/Goods/ as
  uncompressed, mipmapped, bilinear textures (they are anti-aliased 128 px white silhouettes, not pixel art).
* Fonts: writes the importer metas for the TTF and licence files that tools/ui-icons/fetch_fonts.sh
  downloads into Resources/UI/Fonts.

Every .meta gets a deterministic GUID derived from the asset path, so separate worktrees and repeated
runs produce identical files. Standard library only.

Usage: python3 tools/ui-icons/build_icons.py [--check]
  --check  exit 1 if any generated file would change (nothing is written).
"""
import hashlib
import pathlib
import re
import shutil
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
ICONS_JS = ROOT / "docs" / "ui-redesign" / "mockups" / "icons.js"
GOODS_SOURCE = ROOT / "public" / "images" / "game" / "icons"
UI = ROOT / "unity" / "Assets" / "Resources" / "UI"
ICONS = UI / "Icons"
GOODS = ICONS / "Goods"
FONTS = UI / "Fonts"

FONT_NAMES = {
    "Manrope-Regular.ttf": "Manrope",
    "Manrope-SemiBold.ttf": "Manrope SemiBold",
    "Manrope-Bold.ttf": "Manrope",
    "Manrope-ExtraBold.ttf": "Manrope ExtraBold",
    "InstrumentSerif-Italic.ttf": "Instrument Serif",
}

SVG_META = """fileFormatVersion: 2
guid: {guid}
ScriptedImporter:
  internalIDToNameTable: []
  externalObjects: {{}}
  serializedVersion: 2
  userData:
  assetBundleName:
  assetBundleVariant:
  script: {{fileID: 12408, guid: 0000000000000000e000000000000000, type: 0}}
  svgType: 3
  texturedSpriteMeshType: 0
  svgPixelsPerUnit: 100
  gradientResolution: 64
  alignment: 0
  customPivot: {{x: 0, y: 0}}
  generatePhysicsShape: 0
  viewportOptions: 1
  preserveViewport: 0
  advancedMode: 0
  tessellationMode: 1
  predefinedResolutionIndex: 1
  targetResolution: 1080
  resolutionMultiplier: 1
  stepDistance: 10
  samplingStepDistance: 100
  maxCordDeviationEnabled: 0
  maxCordDeviation: 1
  maxTangentAngleEnabled: 0
  maxTangentAngle: 5
  keepTextureAspectRatio: 1
  textureSize: 256
  textureWidth: 256
  textureHeight: 256
  wrapMode: 0
  filterMode: 1
  sampleCount: 4
  preserveSVGImageAspect: 0
  useSVGPixelsPerUnit: 0
  spriteData:
    TessellationDetail: 0
    SpriteName:
    SpritePivot: {{x: 0, y: 0}}
    SpriteAlignment: 0
    SpriteBorder: {{x: 0, y: 0, z: 0, w: 0}}
    SpriteRect:
      serializedVersion: 2
      x: 0
      y: 0
      width: 0
      height: 0
    SpriteID:
    PhysicsOutlines: []
"""

PNG_META = """fileFormatVersion: 2
guid: {guid}
TextureImporter:
  internalIDToNameTable: []
  externalObjects: {{}}
  serializedVersion: 13
  mipmaps:
    mipMapMode: 0
    enableMipMap: 1
    sRGBTexture: 1
    linearTexture: 0
    fadeOut: 0
    borderMipMap: 0
    mipMapsPreserveCoverage: 0
    alphaTestReferenceValue: 0.5
    mipMapFadeDistanceStart: 1
    mipMapFadeDistanceEnd: 3
  bumpmap:
    convertToNormalMap: 0
    externalNormalMap: 0
    heightScale: 0.25
    normalMapFilter: 0
    flipGreenChannel: 0
  isReadable: 0
  streamingMipmaps: 0
  streamingMipmapsPriority: 0
  vTOnly: 0
  ignoreMipmapLimit: 1
  grayScaleToAlpha: 0
  generateCubemap: 6
  cubemapConvolution: 0
  seamlessCubemap: 0
  textureFormat: 1
  maxTextureSize: 2048
  textureSettings:
    serializedVersion: 2
    filterMode: 1
    aniso: 1
    mipBias: 0
    wrapU: 1
    wrapV: 1
    wrapW: 1
  nPOTScale: 0
  lightmap: 0
  compressionQuality: 50
  spriteMode: 0
  spriteExtrude: 1
  spriteMeshType: 1
  alignment: 0
  spritePivot: {{x: 0.5, y: 0.5}}
  spritePixelsToUnits: 100
  spriteBorder: {{x: 0, y: 0, z: 0, w: 0}}
  spriteGenerateFallbackPhysicsShape: 1
  alphaUsage: 1
  alphaIsTransparency: 1
  spriteTessellationDetail: -1
  textureType: 0
  textureShape: 1
  singleChannelComponent: 0
  flipbookRows: 1
  flipbookColumns: 1
  maxTextureSizeSet: 0
  compressionQualitySet: 0
  textureFormatSet: 0
  ignorePngGamma: 0
  applyGammaDecoding: 0
  swizzle: 50462976
  cookieLightType: 0
  platformSettings:
  - serializedVersion: 4
    buildTarget: DefaultTexturePlatform
    maxTextureSize: 2048
    resizeAlgorithm: 0
    textureFormat: -1
    textureCompression: 0
    compressionQuality: 50
    crunchedCompression: 0
    allowsAlphaSplitting: 0
    overridden: 0
    ignorePlatformSupport: 0
    androidETC2FallbackOverride: 0
    forceMaximumCompressionQuality_BC6H_BC7: 0
  spriteSheet:
    serializedVersion: 2
    sprites: []
    outline: []
    customData:
    physicsShape: []
    bones: []
    spriteID:
    internalID: 0
    vertices: []
    indices:
    edges: []
    weights: []
    secondaryTextures: []
    spriteCustomMetadata:
      entries: []
    nameFileIdTable: {{}}
  mipmapLimitGroupName:
  pSDRemoveMatte: 0
  userData:
  assetBundleName:
  assetBundleVariant:
"""

FONT_META = """fileFormatVersion: 2
guid: {guid}
TrueTypeFontImporter:
  externalObjects: {{}}
  serializedVersion: 4
  fontSize: 16
  forceTextureCase: -2
  characterSpacing: 0
  characterPadding: 1
  includeFontData: 1
  fontNames:
  - {family}
  fallbackFontReferences: []
  customCharacters:
  fontRenderingMode: 0
  ascentCalculationMode: 1
  useLegacyBoundsCalculation: 0
  shouldRoundAdvanceValue: 1
  userData:
  assetBundleName:
  assetBundleVariant:
"""

TEXT_META = """fileFormatVersion: 2
guid: {guid}
TextScriptImporter:
  externalObjects: {{}}
  userData:
  assetBundleName:
  assetBundleVariant:
"""


# Unity writes empty scalars as "key: " with a trailing space; keep that so imports leave the metas untouched.
EMPTY_SCALARS = ("userData", "assetBundleName", "assetBundleVariant", "SpriteName", "SpriteID", "customCharacters",
                 "customData", "spriteID", "indices", "mipmapLimitGroupName")


def unity_yaml(text):
    return re.sub(r"(?m)^(\s*(?:" + "|".join(EMPTY_SCALARS) + r")):\s*$", r"\1: ", text)


def guid_for(path):
    relative = path.relative_to(ROOT / "unity").as_posix()
    return hashlib.md5(("idle-cat-forest:" + relative).encode("utf-8")).hexdigest()


def glyphs():
    source = ICONS_JS.read_text(encoding="utf-8")
    body = source.split("window.ICONS = {", 1)[1].split("};", 1)[0]
    found = re.findall(r"^\s*([a-z]+):\s*'([^']*)',?\s*$", body, re.M)
    if not found:
        sys.exit("build_icons: no glyphs found in " + str(ICONS_JS))
    return found


def svg(markup):
    inner = markup.replace('"currentColor"', '"#ffffff"')
    return ('<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" '
            'stroke="#ffffff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">'
            + inner + "</svg>\n")


def planned():
    """Every generated file as (path, bytes)."""
    files = []
    for name, markup in glyphs():
        path = ICONS / (name + ".svg")
        files.append((path, svg(markup).encode("utf-8")))
        files.append((path.with_name(path.name + ".meta"), unity_yaml(SVG_META.format(guid=guid_for(path))).encode("utf-8")))
    for png in sorted(GOODS_SOURCE.glob("*.png")):
        path = GOODS / png.name
        files.append((path, png.read_bytes()))
        files.append((path.with_name(path.name + ".meta"), unity_yaml(PNG_META.format(guid=guid_for(path))).encode("utf-8")))
    for font, family in FONT_NAMES.items():
        path = FONTS / font
        if path.exists():
            files.append((path.with_name(path.name + ".meta"), unity_yaml(FONT_META.format(guid=guid_for(path), family=family)).encode("utf-8")))
    for licence in sorted(FONTS.glob("OFL-*.txt")):
        files.append((licence.with_name(licence.name + ".meta"), unity_yaml(TEXT_META.format(guid=guid_for(licence))).encode("utf-8")))
    return files


def main():
    check = "--check" in sys.argv[1:]
    stale = []
    for path, data in planned():
        if path.exists() and path.read_bytes() == data:
            continue
        stale.append(path)
        if not check:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    for path in stale:
        print(("stale: " if check else "wrote: ") + path.relative_to(ROOT).as_posix())
    if check and stale:
        sys.exit(1)
    print("build_icons: %d file(s) %s" % (len(stale), "out of date" if check else "written"))


if __name__ == "__main__":
    main()
