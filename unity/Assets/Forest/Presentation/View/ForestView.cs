using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Interpolates authoritative positions. Camera motion never moves the simulation.
    /// This file owns the Unity lifecycle, the frame order and the per-domain view hooks; each
    /// domain lives in its own <c>View/ForestView.*.cs</c> partial.
    /// </summary>
    public sealed partial class ForestView : MonoBehaviour
    {
        public Camera Camera { get; private set; }
        public bool DirectControl => ControlledCat != null;
        public Cat ControlledCat => Game.Selected?.Cats.Find(c => c.Alive && c.ControlledBy == Game.PlayerId && c.ControlledBy != "");
        private ForestGame Game => GetComponent<ForestGame>();
        private bool WorldReady => Game.Selected != null && Game.CurrentWorld != null;
        private float nextReconcile, nextHeartbeat;
        private string villageId = "";
        private int hookedLevel = int.MinValue;

        // View hooks. Each domain implements its own in its partial file; unimplemented hooks compile away.
        // Update hooks receive the unscaled frame time; level hooks run when RenderedLevel changes
        // (including the first world frame).
        partial void AwakeLighting();
        partial void UpdateLighting(float dt);
        partial void LevelChangedLighting(int level);
        partial void AwakeTerrain();
        partial void UpdateTerrain(float dt);
        partial void LevelChangedTerrain(int level);
        partial void AwakeProps();
        partial void UpdateProps(float dt);
        partial void LevelChangedProps(int level);
        partial void AwakeFx();
        partial void UpdateFx(float dt);
        partial void LateUpdateFx();
        partial void AwakeMarkers();
        partial void LateUpdateMarkers();
        partial void AwakeAtmosphere();
        partial void UpdateAtmosphere(float dt);
        partial void LateUpdateAtmosphere();
        partial void AwakeCloseUp();
        partial void LateUpdateCloseUp();
        partial void UpdateBuildingMotion(float dt);

        private void Awake()
        {
            worldRoot = new GameObject("Forest inhabitants and places").transform;
            groundRoot = new GameObject("Shared terrain").transform;
            CreateCamera();
            CreateLights();
            CreateSelectionMarker();
            CreateInput();
            AwakeLighting();
            AwakeTerrain();
            AwakeProps();
            AwakeFx();
            AwakeMarkers();
            AwakeAtmosphere();
            AwakeCloseUp();
        }

        private void Update()
        {
            TickInput();
            if (!WorldReady) return;
            if (villageId != Game.Selected.Id)
            {
                ClearWorld(); villageId = Game.Selected.Id; focus = At(Game.Selected.Center); followCatId = ""; SnapManagementCamera();
                if (IntroWanted()) PlayIntro();
            }
            if (!DirectControl && followSelectedCat && SelectedCat != null && SelectedCat.Position.Level != viewedLevel) { viewedLevel = SelectedCat.Position.Level; focus = Position(SelectedCat); nextReconcile = 0; }
            NotifyLevelChange();
            if (Time.unscaledTime >= nextReconcile) { Reconcile(); nextReconcile = Time.unscaledTime + .2f; }
            AnimateCats();
            var controlled = ControlledCat;
            if (controlled != null && Time.unscaledTime >= nextHeartbeat)
            {
                Game.Act(new GameAction { Kind = "KeepCatControl", CatId = controlled.Id });
                nextHeartbeat = Time.unscaledTime + 1;
            }
            ApplyContinuousInput();
            UpdateCamera();
            UpdateHover();
            UpdateSelection();
            float dt = Time.unscaledDeltaTime;
            UpdateLighting(dt);
            UpdateTerrain(dt);
            UpdateProps(dt);
            UpdateFx(dt);
            UpdateAtmosphere(dt);
            UpdateBuildingMotion(dt);
        }

        private void LateUpdate()
        {
            if (!WorldReady) return;
            LateUpdateFx();
            LateUpdateMarkers();
            LateUpdateAtmosphere();
            LateUpdateCloseUp();
        }

        private void NotifyLevelChange()
        {
            int level = RenderedLevel;
            if (level == hookedLevel) return;
            hookedLevel = level;
            LevelChangedLighting(level);
            LevelChangedTerrain(level);
            LevelChangedProps(level);
        }

        private void OnDestroy()
        {
            if (worldRoot != null) Destroy(worldRoot.gameObject);
            if (groundRoot != null) { foreach (var filter in groundRoot.GetComponentsInChildren<MeshFilter>()) Destroy(filter.sharedMesh); Destroy(groundRoot.gameObject); }
            if (selection != null) Destroy(selection.gameObject);
            if (sun != null) Destroy(sun.gameObject);
            if (ownsCamera && Camera != null) Destroy(Camera.gameObject);
            foreach (var material in materials.Values) Destroy(material);
            ReleaseInput();
        }
    }
}
