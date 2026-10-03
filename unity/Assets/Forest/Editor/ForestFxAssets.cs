using System.IO;
using System.Linq;
using IdleCatForest.Presentation;
using UnityEditor;
using UnityEngine;
using Unity.Pipeline.Commands;

namespace IdleCatForest.Editor
{
    /// <summary>
    /// Bakes the effect sprites (drawn by <see cref="FxSprites"/>) and their materials into
    /// <c>Assets/Resources/Fx/</c>: Particles/Standard Unlit materials in Fade and Additive mode, so the
    /// player build includes that shader, and Forest/FxBillboard materials for emotes and blob shadows.
    /// Idempotent: unchanged textures are not rewritten and existing materials are updated in place.
    /// Batch: <c>-executeMethod IdleCatForest.Editor.ForestEditor.FxAssets</c>.
    /// </summary>
    public static partial class ForestEditor
    {
        private const string FxFolder = "Assets/Resources/Fx";

        [MenuItem("Forest/Bake effect assets")]
        [CliCommand("forest_fx_assets", "Bake the effect sprites and particle materials into Resources/Fx", MainThreadRequired = true)]
        public static string FxAssets()
        {
            Directory.CreateDirectory(FxFolder);
            int written = 0;
            foreach (var sprite in FxSprites.All)
                if (BakeSprite(sprite)) written++;
            AssetDatabase.Refresh();
            foreach (var sprite in FxSprites.All) ConfigureSprite(TexturePath(sprite));

            var particle = Shader.Find(FxSprites.ParticleShaderName);
            if (particle == null) throw new System.InvalidOperationException("Shader '" + FxSprites.ParticleShaderName + "' is not available in this editor.");
            var billboard = AssetDatabase.LoadAssetAtPath<Shader>(FxFolder + "/" + FxSprites.BillboardShaderResource + ".shader");
            if (billboard == null) throw new System.InvalidOperationException("Missing " + FxFolder + "/" + FxSprites.BillboardShaderResource + ".shader.");

            int materials = 0;
            foreach (var sprite in FxSprites.Particles) { Material(FxSprites.ParticleMaterialName(sprite), particle, sprite, m => FxSprites.SetupParticleMaterial(m, false)); materials++; }
            foreach (var sprite in FxSprites.Glows) { Material(FxSprites.GlowMaterialName(sprite), particle, sprite, m => FxSprites.SetupParticleMaterial(m, true)); materials++; }
            foreach (var sprite in FxSprites.Billboards) { Material(FxSprites.BillboardMaterialName(sprite), billboard, sprite, m => FxSprites.SetupBillboardMaterial(m, sprite == FxSprites.Blob)); materials++; }
            AssetDatabase.SaveAssets();
            string result = "forest_fx_assets: " + FxSprites.All.Length + " sprites (" + written + " rewritten), " + materials + " materials in " + FxFolder;
            Debug.Log(result);
            return result;
        }

        private static string TexturePath(string sprite) => FxFolder + "/" + FxSprites.TextureName(sprite) + ".png";

        private static bool BakeSprite(string sprite)
        {
            var texture = new Texture2D(FxSprites.Size, FxSprites.Size, TextureFormat.RGBA32, false);
            try
            {
                texture.SetPixels32(FxSprites.Draw(sprite));
                texture.Apply(false, false);
                var bytes = texture.EncodeToPNG();
                string path = TexturePath(sprite);
                if (File.Exists(path) && File.ReadAllBytes(path).SequenceEqual(bytes)) return false;
                File.WriteAllBytes(path, bytes);
                return true;
            }
            finally { Object.DestroyImmediate(texture); }
        }

        private static void ConfigureSprite(string path)
        {
            var importer = (TextureImporter)AssetImporter.GetAtPath(path);
            if (importer == null) throw new System.InvalidOperationException("Texture importer missing for " + path);
            bool dirty = importer.textureType != TextureImporterType.Default || !importer.alphaIsTransparency || !importer.mipmapEnabled
                || importer.wrapMode != TextureWrapMode.Clamp || !importer.sRGBTexture || importer.textureCompression != TextureImporterCompression.Uncompressed
                || importer.filterMode != FilterMode.Bilinear || importer.isReadable;
            if (!dirty) return;
            importer.textureType = TextureImporterType.Default;
            importer.alphaSource = TextureImporterAlphaSource.FromInput;
            importer.alphaIsTransparency = true;
            importer.mipmapEnabled = true;
            importer.wrapMode = TextureWrapMode.Clamp;
            importer.filterMode = FilterMode.Bilinear;
            importer.sRGBTexture = true;
            importer.isReadable = false;
            importer.textureCompression = TextureImporterCompression.Uncompressed;
            importer.SaveAndReimport();
        }

        private static void Material(string name, Shader shader, string sprite, System.Action<Material> setup)
        {
            string path = FxFolder + "/" + name + ".mat";
            var texture = AssetDatabase.LoadAssetAtPath<Texture2D>(TexturePath(sprite));
            var material = AssetDatabase.LoadAssetAtPath<Material>(path);
            bool created = material == null;
            if (created) material = new Material(shader) { name = name };
            else if (material.shader != shader) material.shader = shader;
            material.mainTexture = texture;
            material.color = Color.white;
            setup(material);
            if (created) AssetDatabase.CreateAsset(material, path);
            else EditorUtility.SetDirty(material);
        }
    }
}
