using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Rendering;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The effect sprites, drawn procedurally from signed distance shapes (original art, no external
    /// files), and the materials that render them. <c>forest_fx_assets</c> bakes the same drawings into
    /// <c>Assets/Resources/Fx/</c> so the player ships the textures and the particle shader; at runtime
    /// <see cref="FxAssets"/> loads those and falls back to drawing them in memory.
    /// Sprites are white or light grey with alpha so particle and instance colours tint them; marks that
    /// must read on any ground (Z, "!", drop, heart) carry a dark outline.
    /// </summary>
    public static class FxSprites
    {
        public const string Dot = "dot";
        public const string Glow = "glow";
        public const string Puff = "puff";
        public const string Star = "star";
        public const string Leaf = "leaf";
        public const string Chip = "chip";
        public const string Heart = "heart";
        public const string Paw = "paw";
        public const string Butterfly = "butterfly";
        public const string Zzz = "z";
        public const string Bang = "bang";
        public const string Drop = "drop";
        public const string Blob = "blob";

        /// <summary>Every sprite, in bake order.</summary>
        public static readonly string[] All = { Dot, Glow, Puff, Star, Leaf, Chip, Heart, Paw, Butterfly, Zzz, Bang, Drop, Blob };
        /// <summary>Alpha-blended particle sprites (Particles/Standard Unlit, Fade).</summary>
        public static readonly string[] Particles = { Dot, Puff, Leaf, Chip, Heart, Paw, Butterfly };
        /// <summary>Additive particle sprites (Particles/Standard Unlit, Additive).</summary>
        public static readonly string[] Glows = { Star, Glow, Dot };
        /// <summary>Sprites drawn as instanced billboards or ground quads (Forest/FxBillboard).</summary>
        public static readonly string[] Billboards = { Zzz, Bang, Drop, Heart, Blob };

        public const int Size = 64;
        public const string ResourceFolder = "Fx";
        public const string ParticleShaderName = "Particles/Standard Unlit";
        public const string BillboardShaderName = "Forest/FxBillboard";
        public const string BillboardShaderResource = "FxBillboard";

        public static string TextureName(string sprite) => "fx_" + sprite;
        public static string ParticleMaterialName(string sprite) => "FxParticle_" + sprite;
        public static string GlowMaterialName(string sprite) => "FxGlow_" + sprite;
        public static string BillboardMaterialName(string sprite) => "FxBillboard_" + sprite;

        private static readonly Color Ink = new Color(.082f, .133f, .102f, 1); // #15221a, the guide's dark ring

        /// <summary>Configures a Particles/Standard Unlit material the way its inspector would for Fade or Additive.</summary>
        public static void SetupParticleMaterial(Material m, bool additive)
        {
            m.SetFloat("_Mode", additive ? 4 : 2);
            m.SetOverrideTag("RenderType", "Transparent");
            m.SetFloat("_BlendOp", (float)BlendOp.Add);
            m.SetFloat("_SrcBlend", (float)BlendMode.SrcAlpha);
            m.SetFloat("_DstBlend", additive ? (float)BlendMode.One : (float)BlendMode.OneMinusSrcAlpha);
            m.SetFloat("_ZWrite", 0);
            m.DisableKeyword("_ALPHATEST_ON");
            m.EnableKeyword("_ALPHABLEND_ON");
            m.DisableKeyword("_ALPHAPREMULTIPLY_ON");
            m.DisableKeyword("_ALPHAMODULATE_ON");
            m.renderQueue = (int)RenderQueue.Transparent;
        }

        /// <summary>Configures a Forest/FxBillboard material; ground quads draw before other transparents with a depth bias.</summary>
        public static void SetupBillboardMaterial(Material m, bool ground)
        {
            m.SetFloat("_SrcBlend", (float)BlendMode.SrcAlpha);
            m.SetFloat("_DstBlend", (float)BlendMode.OneMinusSrcAlpha);
            m.SetFloat("_OffsetUnits", ground ? -2 : 0);
            m.enableInstancing = true;
            m.renderQueue = ground ? (int)RenderQueue.Transparent - 10 : (int)RenderQueue.Transparent + 10;
        }

        public static Texture2D CreateTexture(string sprite)
        {
            var texture = new Texture2D(Size, Size, TextureFormat.RGBA32, true) { name = TextureName(sprite), wrapMode = TextureWrapMode.Clamp, filterMode = FilterMode.Bilinear };
            texture.SetPixels32(Draw(sprite));
            texture.Apply(true, false);
            return texture;
        }

        /// <summary>The sprite's pixels, row by row from the bottom, <see cref="Size"/> square.</summary>
        public static Color32[] Draw(string sprite)
        {
            var pixels = new Color32[Size * Size];
            float px = 2f / Size;
            for (int y = 0; y < Size; y++)
                for (int x = 0; x < Size; x++)
                {
                    var p = new Vector2((x + .5f) * px - 1, (y + .5f) * px - 1);
                    pixels[y * Size + x] = Shade(sprite, p, px);
                }
            return pixels;
        }

        private static Color32 Shade(string sprite, Vector2 p, float px)
        {
            float r = p.magnitude;
            switch (sprite)
            {
                case Dot:
                {
                    float a = Mathf.Pow(Mathf.Clamp01(1 - r), 1.6f);
                    return White(a);
                }
                case Glow:
                {
                    float a = Mathf.Clamp01(Mathf.Exp(-r * r * 6) * (1 - Smooth(.85f, 1, r)) + Mathf.Exp(-r * r * 40) * .5f);
                    return White(a);
                }
                case Blob:
                {
                    // A solid core that fades over the outer half, so the visible rim around a body reads.
                    return White(1 - Smooth(.45f, 1, r));
                }
                case Star:
                {
                    float ax = Mathf.Abs(p.x), ay = Mathf.Abs(p.y);
                    float cross = Mathf.Exp(-ax * 16) * Mathf.Exp(-ay * 2.6f) + Mathf.Exp(-ay * 16) * Mathf.Exp(-ax * 2.6f);
                    float core = Mathf.Exp(-r * r * 22);
                    float a = Mathf.Clamp01((cross + core) * (1 - Smooth(.8f, 1, r)));
                    return White(a);
                }
                case Puff:
                {
                    float d = Mathf.Min(Mathf.Min(Circle(p, new Vector2(0, -.05f), .5f), Circle(p, new Vector2(-.38f, -.15f), .36f)),
                        Mathf.Min(Circle(p, new Vector2(.38f, -.12f), .38f), Circle(p, new Vector2(.08f, .32f), .38f)));
                    float a = Mathf.Clamp01(.5f - d / .3f) * .92f;
                    float light = Mathf.Lerp(.78f, 1, Mathf.Clamp01(p.y * .6f + .6f));
                    return Grey(light, a);
                }
                case Leaf:
                {
                    var q = Rotate(p, 35);
                    float d = Vesica(q, .98f, .52f);
                    float fill = Cover(d, px);
                    float rib = Cover(Mathf.Abs(q.x) - .035f, px) * Cover(d + .12f, px);
                    return Grey(Mathf.Lerp(.95f, .62f, rib) * Mathf.Lerp(.85f, 1, Mathf.Clamp01(q.x + .5f)), fill);
                }
                case Chip:
                {
                    var q = Rotate(p, 22);
                    float d = Box(q, new Vector2(.62f, .3f)) - .06f;
                    float fill = Cover(d, px);
                    return Grey(Mathf.Lerp(.7f, 1, Mathf.Clamp01(q.y * 1.4f + .5f)), fill);
                }
                case Paw:
                {
                    float pad = Ellipse(p, new Vector2(0, -.32f), new Vector2(.44f, .34f));
                    float toes = Mathf.Min(Mathf.Min(Circle(p, new Vector2(-.56f, .14f), .19f), Circle(p, new Vector2(-.2f, .48f), .2f)),
                        Mathf.Min(Circle(p, new Vector2(.2f, .48f), .2f), Circle(p, new Vector2(.56f, .14f), .19f)));
                    float d = Mathf.Min(pad, toes);
                    return Outlined(d, .07f, px, 1, .55f);
                }
                case Butterfly:
                {
                    var q = new Vector2(Mathf.Abs(p.x), p.y);
                    float wings = Mathf.Min(Ellipse(q, new Vector2(.4f, .24f), new Vector2(.42f, .36f)), Ellipse(q, new Vector2(.3f, -.32f), new Vector2(.3f, .26f)));
                    float body = Segment(p, new Vector2(0, -.5f), new Vector2(0, .42f)) - .06f;
                    float fill = Cover(wings, px), edge = fill * (1 - Cover(wings + .1f, px)), bodyMask = Cover(body, px);
                    float a = Mathf.Max(fill, bodyMask);
                    if (a <= 0) return White(0);
                    float shade = Mathf.Lerp(1, .55f, edge);
                    var c = Color.Lerp(new Color(shade, shade, shade), Ink, bodyMask);
                    return Rgba(c, a);
                }
                case Heart:
                {
                    const float scale = .66f;
                    var q = new Vector2(p.x * scale, p.y * scale + .55f);
                    float d = HeartSdf(q) / scale;
                    return Outlined(d, .12f, px, 1, 1);
                }
                case Drop:
                {
                    var q = new Vector2(p.x, p.y + .3f) / .95f;
                    float d = UnevenCapsule(q, .5f, .04f, 1.1f) * .95f;
                    var color = Outlined(d, .12f, px, .9f, 1);
                    float shine = Cover(Ellipse(p, new Vector2(-.17f, -.2f), new Vector2(.1f, .17f)), px);
                    if (shine > 0) { var c = (Color)color; c.r = c.g = c.b = Mathf.Lerp(c.r, 1, shine); return c; }
                    return color;
                }
                case Zzz:
                {
                    float d = Mathf.Min(Mathf.Min(Segment(p, new Vector2(-.46f, .5f), new Vector2(.46f, .5f)), Segment(p, new Vector2(.46f, .5f), new Vector2(-.46f, -.5f))),
                        Segment(p, new Vector2(-.46f, -.5f), new Vector2(.46f, -.5f))) - .13f;
                    return Outlined(d, .13f, px, 1, 1);
                }
                case Bang:
                {
                    float disk = Circle(p, Vector2.zero, .74f);
                    float glyph = Mathf.Min(Segment(p, new Vector2(0, .4f), new Vector2(0, -.06f)) - .12f, Circle(p, new Vector2(0, -.36f), .12f));
                    float outer = Cover(disk - .12f, px), fill = Cover(disk, px), mark = Cover(glyph, px);
                    if (outer <= 0) return White(0);
                    var c = Color.Lerp(Ink, Color.white, fill * (1 - mark));
                    return Rgba(c, outer);
                }
                default:
                    return White(0);
            }
        }

        // Fill with a dark outline of width w behind it (white or grey fill so tints colour it).
        private static Color32 Outlined(float d, float w, float px, float fillShade, float outlineAlpha)
        {
            float outer = Cover(d - w, px) * outlineAlpha, fill = Cover(d, px);
            float a = Mathf.Max(outer, fill);
            if (a <= 0) return White(0);
            var c = Color.Lerp(Ink, new Color(fillShade, fillShade, fillShade), fill / Mathf.Max(a, 1e-4f));
            return Rgba(c, a);
        }

        private static Color32 White(float a) => Rgba(Color.white, a);
        private static Color32 Grey(float shade, float a) => Rgba(new Color(shade, shade, shade), a);
        private static Color32 Rgba(Color c, float a) { c.a = Mathf.Clamp01(a); return c; }
        private static float Cover(float d, float px) => Mathf.Clamp01(.5f - d / px);
        private static float Smooth(float a, float b, float x) { float t = Mathf.Clamp01((x - a) / (b - a)); return t * t * (3 - 2 * t); }
        private static float Circle(Vector2 p, Vector2 c, float r) => (p - c).magnitude - r;
        private static float Ellipse(Vector2 p, Vector2 c, Vector2 radii) { var q = new Vector2((p.x - c.x) / radii.x, (p.y - c.y) / radii.y); return (q.magnitude - 1) * Mathf.Min(radii.x, radii.y); }
        private static Vector2 Rotate(Vector2 p, float degrees) { float a = degrees * Mathf.Deg2Rad, c = Mathf.Cos(a), s = Mathf.Sin(a); return new Vector2(c * p.x + s * p.y, -s * p.x + c * p.y); }
        private static float Box(Vector2 p, Vector2 b) { var d = new Vector2(Mathf.Abs(p.x) - b.x, Mathf.Abs(p.y) - b.y); return new Vector2(Mathf.Max(d.x, 0), Mathf.Max(d.y, 0)).magnitude + Mathf.Min(Mathf.Max(d.x, d.y), 0); }
        private static float Segment(Vector2 p, Vector2 a, Vector2 b) { var pa = p - a; var ba = b - a; float h = Mathf.Clamp01(Vector2.Dot(pa, ba) / Vector2.Dot(ba, ba)); return (pa - ba * h).magnitude; }

        private static float Vesica(Vector2 p, float r, float d)
        {
            p = new Vector2(Mathf.Abs(p.x), Mathf.Abs(p.y));
            float b = Mathf.Sqrt(r * r - d * d);
            return (p.y - b) * d > p.x * b ? (p - new Vector2(0, b)).magnitude : (p - new Vector2(-d, 0)).magnitude - r;
        }

        private static float HeartSdf(Vector2 p)
        {
            p.x = Mathf.Abs(p.x);
            if (p.y + p.x > 1) return (p - new Vector2(.25f, .75f)).magnitude - Mathf.Sqrt(2) / 4;
            float a = (p - new Vector2(0, 1)).sqrMagnitude;
            float m = .5f * Mathf.Max(p.x + p.y, 0);
            float b = (p - new Vector2(m, m)).sqrMagnitude;
            return Mathf.Sqrt(Mathf.Min(a, b)) * Mathf.Sign(p.x - p.y);
        }

        private static float UnevenCapsule(Vector2 p, float r1, float r2, float h)
        {
            p.x = Mathf.Abs(p.x);
            float b = (r1 - r2) / h, a = Mathf.Sqrt(1 - b * b);
            float k = Vector2.Dot(p, new Vector2(-b, a));
            if (k < 0) return p.magnitude - r1;
            if (k > a * h) return (p - new Vector2(0, h)).magnitude - r2;
            return Vector2.Dot(p, new Vector2(a, b)) - r1;
        }
    }

    /// <summary>
    /// Loads the baked effect materials from <c>Resources/Fx</c>, or builds equivalents in memory when the
    /// bake is missing. Owns and destroys only what it created.
    /// </summary>
    internal sealed class FxAssets : IDisposable
    {
        private readonly Dictionary<string, Material> cache = new Dictionary<string, Material>();
        private readonly Dictionary<string, Texture2D> textures = new Dictionary<string, Texture2D>();
        private readonly List<UnityEngine.Object> owned = new List<UnityEngine.Object>();
        private Shader particleShader, billboardShader;
        private bool searchedParticle, searchedBillboard;

        /// <summary>True when at least the particle shader is available (bursts and ambient life can render).</summary>
        public bool ParticlesAvailable => Particle(FxSprites.Dot) != null;

        public Material Particle(string sprite) => Get(FxSprites.ParticleMaterialName(sprite), sprite, Kind.Particle);
        public Material Glow(string sprite) => Get(FxSprites.GlowMaterialName(sprite), sprite, Kind.Glow);
        public Material Billboard(string sprite) => Get(FxSprites.BillboardMaterialName(sprite), sprite, Kind.Billboard);

        private enum Kind { Particle, Glow, Billboard }

        private Material Get(string name, string sprite, Kind kind)
        {
            if (cache.TryGetValue(name, out var material)) return material;
            material = Resources.Load<Material>(FxSprites.ResourceFolder + "/" + name);
            if (material == null)
            {
                var shader = kind == Kind.Billboard ? BillboardShader() : ParticleShader();
                if (shader != null)
                {
                    material = new Material(shader) { name = name + " (runtime)", mainTexture = Texture(sprite) };
                    if (kind == Kind.Billboard) FxSprites.SetupBillboardMaterial(material, sprite == FxSprites.Blob);
                    else FxSprites.SetupParticleMaterial(material, kind == Kind.Glow);
                    owned.Add(material);
                }
            }
            cache[name] = material;
            return material;
        }

        private Texture2D Texture(string sprite)
        {
            if (textures.TryGetValue(sprite, out var texture)) return texture;
            texture = Resources.Load<Texture2D>(FxSprites.ResourceFolder + "/" + FxSprites.TextureName(sprite));
            if (texture == null) { texture = FxSprites.CreateTexture(sprite); owned.Add(texture); }
            textures[sprite] = texture;
            return texture;
        }

        private Shader ParticleShader()
        {
            if (!searchedParticle) { particleShader = Shader.Find(FxSprites.ParticleShaderName); searchedParticle = true; }
            return particleShader;
        }

        private Shader BillboardShader()
        {
            if (!searchedBillboard)
            {
                billboardShader = Resources.Load<Shader>(FxSprites.ResourceFolder + "/" + FxSprites.BillboardShaderResource);
                if (billboardShader == null) billboardShader = Shader.Find(FxSprites.BillboardShaderName);
                searchedBillboard = true;
            }
            return billboardShader;
        }

        public void Dispose()
        {
            foreach (var item in owned) if (item != null) UnityEngine.Object.Destroy(item);
            owned.Clear(); cache.Clear(); textures.Clear();
        }
    }
}
