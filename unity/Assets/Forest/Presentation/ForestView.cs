using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.InputSystem;

namespace IdleCatForest.Presentation
{
    /// <summary>Interpolates authoritative positions. Camera motion never moves the simulation.</summary>
    public sealed class ForestView : MonoBehaviour
    {
        public Camera Camera { get; private set; }
        public string SelectedCatId { get; private set; } = "";
        public string SelectedBuildingId { get; private set; } = "";
        public Int2 CursorTile { get; private set; }
        public bool DirectControl => ControlledCat != null;
        public float ManagementZoom => DirectControl ? managementZoom : zoom;
        public void ZoomBy(float steps)
        {
            if (DirectControl || float.IsNaN(steps) || float.IsInfinity(steps)) return;
            // Input System 1.x uses normalized wheel steps, including fractional trackpad motion.
            zoom = Mathf.Clamp(zoom * Mathf.Exp(Mathf.Clamp(-steps * .12f, -20, 20)), 3.5f, 36);
            ApplyManagementCamera();
        }
        public Cat ControlledCat => Game.Selected?.Cats.Find(c => c.Alive && c.ControlledBy == Game.PlayerId && c.ControlledBy != "");
        public Cat SelectedCat => Game.Selected?.Cats.Find(c => c.Id == SelectedCatId);
        public Building SelectedBuilding => Game.Selected?.Buildings.Find(b => b.Id == SelectedBuildingId);
        public event Action<Int2> TileClicked;
        private ForestGame Game => GetComponent<ForestGame>();
        private readonly Dictionary<string, GameObject> entities = new Dictionary<string, GameObject>();
        private readonly Dictionary<string, string> entityAssets = new Dictionary<string, string>();
        private readonly Dictionary<string, GameObject> models = new Dictionary<string, GameObject>();
        private readonly Dictionary<string, Material> materials = new Dictionary<string, Material>();
        private readonly HashSet<string> touched = new HashSet<string>();
        private Transform worldRoot, groundRoot;
        private Vector3 focus, managementFocus;
        private Int2 groundCenter;
        private float zoom = 14, managementZoom = 14, yaw, nextInput, nextReconcile, nextHeartbeat;
        private bool wasDirect;
        private string villageId = "";
        private int terrainSignature;
        private LineRenderer selection;
        private Light sun;
        private bool ownsCamera;

        private void Awake()
        {
            worldRoot = new GameObject("Forest inhabitants and places").transform;
            groundRoot = new GameObject("Shared terrain").transform;
            Camera = UnityEngine.Camera.main;
            if (Camera == null) { var go = new GameObject("Forest camera"); Camera = go.AddComponent<Camera>(); go.tag = "MainCamera"; ownsCamera = true; }
            Camera.clearFlags = CameraClearFlags.SolidColor;
            Camera.backgroundColor = new Color(.11f, .18f, .17f);
            Camera.nearClipPlane = .1f; Camera.farClipPlane = 240;
            Camera.allowHDR = false; Camera.allowMSAA = true;
            var existingLights = FindObjectsByType<Light>();
            foreach (var light in existingLights) light.enabled = false;
            sun = new GameObject("Afternoon light").AddComponent<Light>();
            sun.type = LightType.Directional; sun.color = new Color(1, .89f, .72f); sun.intensity = 1.35f;
            sun.transform.rotation = Quaternion.Euler(48, -32, 0);
            sun.shadows = LightShadows.Soft; sun.shadowStrength = .55f; sun.shadowBias = .08f;
            RenderSettings.ambientMode = UnityEngine.Rendering.AmbientMode.Flat;
            RenderSettings.ambientLight = new Color(.52f, .62f, .58f);
            RenderSettings.fog = true; RenderSettings.fogColor = Camera.backgroundColor;
            RenderSettings.fogMode = FogMode.Linear; RenderSettings.fogStartDistance = 45; RenderSettings.fogEndDistance = 110;
            QualitySettings.shadowDistance = 45;
            selection = new GameObject("Selected place").AddComponent<LineRenderer>();
            selection.material = Material("selection", new Color(1, .78f, .38f), true);
            selection.widthMultiplier = .045f; selection.loop = true; selection.positionCount = 4;
            selection.shadowCastingMode = UnityEngine.Rendering.ShadowCastingMode.Off;
            selection.receiveShadows = false;
        }

        public void FocusVillage()
        {
            var v = Game.Selected; if (v == null) return;
            var center = new Vector3(v.Center.X, 0, v.Center.Z);
            if (DirectControl) { managementFocus = center; managementZoom = Mathf.Max(10, v.Radius + 5); return; }
            focus = center; zoom = Mathf.Max(10, v.Radius + 5); villageId = "";
            ApplyManagementCamera();
            SelectedCatId = ""; SelectedBuildingId = "";
        }

        public void InspectCat(string id) { SelectedCatId = id; SelectedBuildingId = ""; var cat = SelectedCat; if (cat != null && !DirectControl) focus = Position(cat); }
        public void InspectBuilding(string id) { SelectedBuildingId = id; SelectedCatId = ""; var b = SelectedBuilding; if (b != null && !DirectControl) focus = At(b.Position) + new Vector3((b.Width - 1) * .5f, 0, (b.Depth - 1) * .5f); }
        public void EnterSelectedCat() { if (SelectedCat != null) Game.Act(new GameAction { Kind = "EnterCatControl", CatId = SelectedCat.Id }); }
        public void LeaveCat() { var cat = ControlledCat; if (cat != null) Game.Act(new GameAction { Kind = "LeaveCatControl", CatId = cat.Id }); }

        private void Update()
        {
            if (Game.Selected == null || Game.CurrentWorld == null) return;
            if (villageId != Game.Selected.Id) { ClearWorld(); villageId = Game.Selected.Id; focus = At(Game.Selected.Center); }
            if (Time.unscaledTime >= nextReconcile) { Reconcile(); nextReconcile = Time.unscaledTime + .2f; }
            AnimateCats();
            var controlled = ControlledCat;
            if (controlled != null && Time.unscaledTime >= nextHeartbeat)
            {
                Game.Act(new GameAction { Kind = "KeepCatControl", CatId = controlled.Id });
                nextHeartbeat = Time.unscaledTime + 1;
            }
            ReadInput();
            UpdateCamera();
            UpdateSelection();
        }

        private void ReadInput()
        {
            var mouse = Mouse.current; var keyboard = Keyboard.current;
            if (mouse == null || keyboard == null) return;
            var ray = Camera.ScreenPointToRay(mouse.position.ReadValue());
            var plane = new Plane(Vector3.up, Vector3.zero);
            if (plane.Raycast(ray, out float distance)) { var point = ray.GetPoint(distance); CursorTile = new Int2(Mathf.RoundToInt(point.x), Mathf.RoundToInt(point.z)); }
            bool onUI = Game.UI != null && Game.UI.PointerOverPanel;
            bool typing = Game.UI != null && Game.UI.TextInputFocused;
            if (typing) return;
            if (keyboard.escapeKey.wasPressedThisFrame) { if (DirectControl) LeaveCat(); else Game.UI?.ClosePanel(); }
            if (keyboard.tabKey.wasPressedThisFrame) { if (DirectControl) LeaveCat(); else EnterSelectedCat(); }
            var controlled = ControlledCat;
            if (controlled != null)
            {
                if (!onUI && mouse.rightButton.isPressed) yaw += mouse.delta.ReadValue().x * .2f;
                if (keyboard.eKey.wasPressedThisFrame)
                {
                    var pile = Game.Selected.Stockpiles.Where(p => Int2.Distance(p.Position, controlled.Position) <= 1).OrderBy(p => Int2.Distance(p.Position, controlled.Position)).FirstOrDefault();
                    Game.Act(new GameAction { Kind = "InteractCat", CatId = controlled.Id, Position = CursorTile, TargetId = pile?.Id ?? "", Resource = pile?.Report.FirstOrDefault(s => s.Amount > 0)?.Resource ?? "food", Amount = 1 });
                }
                bool pressed = keyboard.wKey.wasPressedThisFrame || keyboard.aKey.wasPressedThisFrame || keyboard.sKey.wasPressedThisFrame || keyboard.dKey.wasPressedThisFrame || keyboard.upArrowKey.wasPressedThisFrame || keyboard.downArrowKey.wasPressedThisFrame || keyboard.leftArrowKey.wasPressedThisFrame || keyboard.rightArrowKey.wasPressedThisFrame;
                if (Time.unscaledTime >= nextInput || pressed)
                {
                    int x = (keyboard.dKey.isPressed || keyboard.dKey.wasPressedThisFrame || keyboard.rightArrowKey.isPressed || keyboard.rightArrowKey.wasPressedThisFrame ? 1 : 0) - (keyboard.aKey.isPressed || keyboard.aKey.wasPressedThisFrame || keyboard.leftArrowKey.isPressed || keyboard.leftArrowKey.wasPressedThisFrame ? 1 : 0);
                    int z = (keyboard.wKey.isPressed || keyboard.wKey.wasPressedThisFrame || keyboard.upArrowKey.isPressed || keyboard.upArrowKey.wasPressedThisFrame ? 1 : 0) - (keyboard.sKey.isPressed || keyboard.sKey.wasPressedThisFrame || keyboard.downArrowKey.isPressed || keyboard.downArrowKey.wasPressedThisFrame ? 1 : 0);
                    if (x != 0 || z != 0)
                    {
                        var direction = Quaternion.Euler(0, yaw, 0) * new Vector3(x, 0, z);
                        var step = Mathf.Abs(direction.x) > Mathf.Abs(direction.z) ? new Int2(Math.Sign(direction.x), 0) : new Int2(0, Math.Sign(direction.z));
                        Game.Act(new GameAction { Kind = "MoveCat", CatId = controlled.Id, Position = step });
                    }
                    nextInput = Time.unscaledTime + .1f;
                }
            }
            else
            {
                float x = (keyboard.dKey.isPressed || keyboard.rightArrowKey.isPressed ? 1 : 0) - (keyboard.aKey.isPressed || keyboard.leftArrowKey.isPressed ? 1 : 0);
                float z = (keyboard.wKey.isPressed || keyboard.upArrowKey.isPressed ? 1 : 0) - (keyboard.sKey.isPressed || keyboard.downArrowKey.isPressed ? 1 : 0);
                var forward = Vector3.ProjectOnPlane(Camera.transform.forward, Vector3.up).normalized;
                var direction = Vector3.ClampMagnitude(Camera.transform.right * x + forward * z, 1);
                focus += direction * Time.unscaledDeltaTime * zoom * 1.1f;
                if (!onUI)
                {
                    float steps = mouse.scroll.ReadValue().y;
                    if (steps != 0)
                    {
                        var cursor = mouse.position.ReadValue(); var anchor = GroundPoint(cursor);
                        ZoomBy(steps);
                        focus += anchor - GroundPoint(cursor);
                        ApplyManagementCamera();
                    }
                    if (mouse.middleButton.isPressed || mouse.rightButton.isPressed)
                    {
                        var cursor = mouse.position.ReadValue();
                        focus += GroundPoint(cursor - mouse.delta.ReadValue()) - GroundPoint(cursor);
                    }
                }
                if (keyboard.equalsKey.wasPressedThisFrame || keyboard.numpadPlusKey.wasPressedThisFrame) ZoomBy(1);
                if (keyboard.minusKey.wasPressedThisFrame || keyboard.numpadMinusKey.wasPressedThisFrame) ZoomBy(-1);
                if (keyboard.homeKey.wasPressedThisFrame) FocusVillage();
            }
            if (mouse.leftButton.wasPressedThisFrame && !onUI)
            {
                if (Game.UI != null && Game.UI.HasPlacement) { TileClicked?.Invoke(CursorTile); return; }
                var cat = Game.Selected.Cats.Where(c => c.Alive).OrderBy(c => Vector2.Distance(new Vector2((float)c.X, (float)c.Z), new Vector2(CursorTile.X, CursorTile.Z))).FirstOrDefault();
                if (cat != null && Vector2.Distance(new Vector2((float)cat.X, (float)cat.Z), new Vector2(CursorTile.X, CursorTile.Z)) < .8f) { InspectCat(cat.Id); Game.UI?.OpenPanel("Inspect"); }
                else
                {
                    var b = Game.Selected.Buildings.Find(p => CursorTile.X >= p.Position.X && CursorTile.X < p.Position.X + p.Width && CursorTile.Z >= p.Position.Z && CursorTile.Z < p.Position.Z + p.Depth);
                    InspectBuilding(b?.Id ?? "");
                    if (b != null) Game.UI?.OpenPanel("Inspect");
                }
            }
        }

        private Vector3 GroundPoint(Vector2 screen)
        {
            var ray = Camera.ScreenPointToRay(screen);
            return new Plane(Vector3.up, Vector3.zero).Raycast(ray, out float distance) ? ray.GetPoint(distance) : focus;
        }

        private void ApplyManagementCamera()
        {
            Camera.orthographic = true; Camera.orthographicSize = zoom;
            Camera.transform.rotation = Quaternion.Euler(55, 0, 0);
            float covered = Game.UI?.WorldLeftFraction ?? 0;
            Camera.transform.position = focus - Camera.transform.forward * 50 - Camera.transform.right * (zoom * Camera.aspect * covered);
        }

        private void UpdateCamera()
        {
            var cat = ControlledCat;
            if (cat != null)
            {
                if (!wasDirect) { managementFocus = focus; managementZoom = zoom; }
                Camera.orthographic = false; Camera.fieldOfView = 58;
                var target = Position(cat) + Vector3.up * .55f;
                var offset = Quaternion.Euler(0, yaw, 0) * new Vector3(0, 2.5f, -3.8f);
                Camera.transform.position = Vector3.Lerp(Camera.transform.position, target + offset, 1 - Mathf.Exp(-Time.unscaledDeltaTime * 8));
                Camera.transform.LookAt(target);
                RenderSettings.fogStartDistance = 24; RenderSettings.fogEndDistance = 58;
            }
            else
            {
                if (wasDirect) { focus = managementFocus; zoom = managementZoom; }
                ApplyManagementCamera();
                RenderSettings.fogStartDistance = 65; RenderSettings.fogEndDistance = 120;
            }
            wasDirect = cat != null;
        }

        private void Reconcile()
        {
            var world = Game.CurrentWorld; var v = Game.Selected;
            touched.Clear();
            int signature = v.Known.Count * 397 + world.Tiles.Count;
            foreach (var tile in world.Tiles) if (tile.Water || tile.Mountain || tile.Dirt) signature = unchecked(signature * 31 + tile.Position.GetHashCode() + (tile.Water ? 1 : tile.Mountain ? 2 : 3));
            var center = ControlledCat == null ? focus : Position(ControlledCat);
            groundCenter = new Int2(Mathf.RoundToInt(center.x / 8) * 8, Mathf.RoundToInt(center.z / 8) * 8);
            signature = unchecked(signature * 397 + groundCenter.GetHashCode());
            if (signature != terrainSignature) { MakeGround(); terrainSignature = signature; }
            foreach (var b in v.Buildings)
            {
                var go = Entity("building:" + b.Id, b.Kind, At(b.Position) + new Vector3((b.Width - 1) * .5f, 0, (b.Depth - 1) * .5f), Mathf.Min(b.Width, b.Depth) * .92f);
                Int2? entrance = b.HasEntrance ? b.Entrance : (Int2?)null;
                if (entrance.HasValue)
                {
                    var p = entrance.Value;
                    var outward = p.X < b.Position.X ? Vector3.left : p.X >= b.Position.X + b.Width ? Vector3.right : p.Z < b.Position.Z ? Vector3.back : Vector3.forward;
                    go.transform.rotation = Quaternion.LookRotation(outward);
                    Entity("entrance:" + b.Id, "road", At(p) - outward * .45f + Vector3.up * .015f, .52f);
                }
                if (!b.Completed) go.transform.localScale = new Vector3(go.transform.localScale.x, Mathf.Max(.15f, (float)b.Progress / Mathf.Max(1, (float)b.RequiredWork)) * go.transform.localScale.x, go.transform.localScale.z);
                else if (b.WorkerId != "" && !b.Paused && b.BlockedReason == "")
                    Entity("activity:" + b.Id, "cargo_tools", go.transform.position + new Vector3(.3f, .7f, .1f), .3f);
            }
            foreach (var pile in v.Stockpiles)
            {
                if (pile.Kind != "storage") continue;
                Entity("pile:" + pile.Id, "stockpile", At(pile.Position), 1.25f);
                int i = 0;
                foreach (var stack in pile.Report.Where(s => s.Amount > 0).Take(5))
                {
                    Entity("pilegoods:" + pile.Id + ":" + stack.Resource, "cargo_" + stack.Resource.ToLowerInvariant(), At(pile.Position) + new Vector3((i % 2) * .35f - .2f, .35f + i / 2 * .18f, 0), .32f); i++;
                }
            }
            foreach (var f in v.Farms)
                for (int x = 0; x < f.Width; x++) for (int z = 0; z < f.Depth; z++)
                    Entity("farm:" + f.Id + ":" + x + ":" + z, f.Crop == "grain" ? "grain_plot" : f.Crop == "catnip" ? "catnip_plot" : "herb_plot", At(new Int2(f.Position.X + x, f.Position.Z + z)), .85f + (float)Math.Min(1, f.Growth) * .15f);
            var known = new HashSet<Int2>(v.Known);
            var tiles = world.Tiles.ToDictionary(t => t.Position);
            bool Wall(Int2 p) => known.Contains(p) && tiles.TryGetValue(p, out var t) && t.Wall;
            foreach (var tile in world.Tiles)
            {
                if (Int2.Distance(tile.Position, groundCenter) > 76 || !known.Contains(tile.Position)) continue;
                if (tile.Wall)
                {
                    Entity("fence:post:" + tile.Position, "fence_post", At(tile.Position), 1, naturalScale: true);
                    foreach (var step in new[] { new Int2(1, 0), new Int2(0, 1) })
                    {
                        var next = new Int2(tile.Position.X + step.X, tile.Position.Z + step.Z);
                        if (!Wall(next)) continue;
                        var rail = Entity("fence:rail:" + tile.Position + ":" + next, "fence_rail", (At(tile.Position) + At(next)) * .5f, 1, naturalScale: true);
                        rail.transform.rotation = Quaternion.Euler(0, step.X == 0 ? 90 : 0, 0);
                    }
                    continue;
                }
                string asset = TileAsset(tile);
                if (asset != "")
                {
                    float width = tile.Road || tile.Overlay == "road_built" || tile.Rail || tile.Bridge ? 1 : asset == "tree_oak" ? 1.7f : .85f;
                    var foliage = Entity("tile:" + tile.Position, asset, At(tile.Position), width);
                    if (asset == "berry_bush" || asset == "shrub") foliage.transform.localScale = new Vector3(width, width * .5f, width);
                }
                int dx = tile.Position.X - v.Center.X, dz = tile.Position.Z - v.Center.Z;
                bool cardinalGate = v.LayoutVersion > 0 && (dx == 0 && Math.Abs(dz) == v.Radius || dz == 0 && Math.Abs(dx) == v.Radius);
                if (tile.Road || tile.Dirt || tile.Overlay == "road_built" || cardinalGate)
                {
                    bool horizontal = Wall(new Int2(tile.Position.X - 1, tile.Position.Z)) && Wall(new Int2(tile.Position.X + 1, tile.Position.Z));
                    bool vertical = Wall(new Int2(tile.Position.X, tile.Position.Z - 1)) && Wall(new Int2(tile.Position.X, tile.Position.Z + 1));
                    if (horizontal || vertical)
                    {
                        var gate = Entity("gate:" + tile.Position, "gate", At(tile.Position), 1, naturalScale: true);
                        gate.transform.rotation = Quaternion.Euler(0, horizontal ? 0 : 90, 0);
                    }
                }
            }
            foreach (var edge in v.BoundaryEdges)
            {
                if (!known.Contains(edge.From) && !known.Contains(edge.To)) continue;
                var go = Entity("boundary:" + edge.From + ":" + edge.To, "fence", (At(edge.From) + At(edge.To)) * .5f, 1.05f);
                go.transform.rotation = Quaternion.Euler(0, edge.From.X == edge.To.X ? 0 : 90, 0);
            }
            var carriedItems = v.Items.ToLookup(item => item.LocationId);
            var awaitingPickup = new HashSet<string>(v.Jobs.Where(job => !job.Completed && job.Phase == "item_fetch").Select(job => job.Id));
            foreach (var c in v.Cats.Where(c => c.Alive))
            {
                var go = Entity("cat:" + c.Id, "cat", Position(c), .85f, false);
                var visual = go.GetComponent<CatMotion>();
                if (visual == null) { visual = go.AddComponent<CatMotion>(); Coat(go, c.Id); }
                visual.CatId = c.Id;
                if (c.Cargo.Count > 0 && c.Cargo[0].Amount > 0)
                    Entity("cargo:" + c.Id, "cargo_" + c.Cargo[0].Resource.ToLowerInvariant(), go.transform.position + Vector3.up * .6f, .3f);
                else if (c.JobId != "" && !awaitingPickup.Contains(c.JobId))
                {
                    var item = carriedItems[c.JobId].FirstOrDefault();
                    if (item != null) Entity("cargo:" + c.Id, ItemCargoAsset(item.Kind), go.transform.position + Vector3.up * .6f, .3f);
                }
            }
            foreach (var vehicle in v.Vehicles) Entity("vehicle:" + vehicle.Id, vehicle.Mode == "shipping" ? "boat" : "cart", At(vehicle.Position), 1);
            foreach (var trade in world.TradeOffers.Where(t => t.Status == "outbound" || t.Status == "returning" || t.Status == "travelling"))
                if (trade.Path.Count > 0) Entity("trade:" + trade.Id, "cart", At(trade.Path[Mathf.Clamp(trade.PathIndex, 0, trade.Path.Count - 1)]), .8f);
            foreach (var raid in v.Raids) { var go = Entity("raid:" + raid.Id, "cat", At(raid.Position), 1.1f); Tint(go, Material("raider", new Color(.46f, .18f, .16f))); }
            foreach (var key in entities.Keys.Where(k => !touched.Contains(k)).ToArray()) { Destroy(entities[key]); entities.Remove(key); entityAssets.Remove(key); }
        }

        private static string ItemCargoAsset(string kind) => kind switch
        {
            "tool" => "cargo_tools", "weapon" => "cargo_weapons", "armor" => "cargo_armor",
            "mug" => "cargo_brew", "bowl" => "water_bowl", "furniture" => "cargo_lumber",
            "clothing" => "cargo_cloth", "trinket" => "cargo_gem", "toy" => "cargo_bone",
            "brick" => "cargo_blocks", _ => "cargo_materials"
        };

        private string TileAsset(Tile tile)
        {
            if (tile.Wall) return "fence";
            if (tile.Dock) return "dock";
            if (tile.Rail) return "rail";
            if (tile.Bridge) return "road";
            if (tile.Road || tile.Overlay == "road_built") return "road";
            if (tile.Amount <= 0) return "";
            switch (tile.Resource.ToLowerInvariant())
            {
                case "logs": case "wood": return "tree_oak";
                case "materials": case "stone": return "rock";
                case "ore": case "iron": return "ore_iron";
                case "coal": return "ore_coal";
                case "food": return "berry_bush";
                case "herbs": return "herb_plot";
                case "fibre": return "shrub";
                case "water": case "fish": return tile.Water ? "" : "reeds";
                default: return "rock";
            }
        }

        private void MakeGround()
        {
            foreach (Transform child in groundRoot) { var filter = child.GetComponent<MeshFilter>(); if (filter != null) Destroy(filter.sharedMesh); Destroy(child.gameObject); }
            var v = Game.Selected; var known = new HashSet<Int2>(v.Known);
            var terrain = Game.CurrentWorld.Tiles.ToDictionary(tile => tile.Position);
            var groups = new Dictionary<int, List<Vector3>>();
            for (int z = groundCenter.Z - 38; z <= groundCenter.Z + 38; z++) for (int x = groundCenter.X - 38; x <= groundCenter.X + 38; x++)
            {
                var point = new Int2(x, z); terrain.TryGetValue(point, out var tile);
                int category = !known.Contains(point) ? 0 : tile != null && tile.Water ? 1 : tile != null && tile.Mountain ? 2 : tile != null && tile.Dirt ? 6 : 3 + Math.Abs(unchecked(x * 73 + z * 37)) % 3;
                if (!groups.ContainsKey(category)) groups[category] = new List<Vector3>();
                groups[category].Add(new Vector3(x, 0, z));
            }
            Color[] colors = { new Color(.105f, .18f, .17f), new Color(.18f, .38f, .40f), new Color(.39f, .43f, .39f), new Color(.36f, .47f, .30f), new Color(.367f, .477f, .305f), new Color(.363f, .472f, .302f), new Color(.40f, .43f, .29f) };
            foreach (var pair in groups)
            {
                var vertices = new List<Vector3>(); var triangles = new List<int>();
                foreach (var p in pair.Value)
                {
                    int n = vertices.Count; float h = pair.Key == 1 ? -.07f : -.025f;
                    vertices.Add(p + new Vector3(-.5f, h, -.5f)); vertices.Add(p + new Vector3(-.5f, h, .5f)); vertices.Add(p + new Vector3(.5f, h, .5f)); vertices.Add(p + new Vector3(.5f, h, -.5f));
                    triangles.AddRange(new[] { n, n + 1, n + 2, n, n + 2, n + 3 });
                }
                var mesh = new Mesh { name = "Forest ground " + pair.Key }; mesh.SetVertices(vertices); mesh.SetTriangles(triangles, 0); mesh.RecalculateNormals(); mesh.RecalculateBounds();
                var go = new GameObject("Ground " + pair.Key, typeof(MeshFilter), typeof(MeshRenderer)); go.transform.SetParent(groundRoot);
                go.GetComponent<MeshFilter>().sharedMesh = mesh; go.GetComponent<MeshRenderer>().sharedMaterial = Material("ground:" + pair.Key, colors[pair.Key]);
            }
        }

        private GameObject Entity(string id, string asset, Vector3 position, float width, bool move = true, bool naturalScale = false)
        {
            touched.Add(id);
            if (entityAssets.TryGetValue(id, out var previous) && previous != asset)
            { Destroy(entities[id]); entities.Remove(id); entityAssets.Remove(id); }
            if (!entities.TryGetValue(id, out var go))
            {
                if (!models.TryGetValue(asset, out var model)) { model = Resources.Load<GameObject>("ForestArt/" + asset); models[asset] = model; }
                go = new GameObject(id); go.transform.SetParent(worldRoot); go.transform.position = position;
                if (model != null)
                {
                    var child = Instantiate(model, go.transform); child.name = asset;
                    var renderers = child.GetComponentsInChildren<Renderer>();
                    if (renderers.Length > 0 && !naturalScale)
                    {
                        var bounds = renderers[0].bounds; foreach (var r in renderers) bounds.Encapsulate(r.bounds);
                        float scale = 1 / Mathf.Max(.01f, Mathf.Max(bounds.size.x, bounds.size.z));
                        child.transform.localScale *= scale;
                        child.transform.localPosition -= new Vector3(bounds.center.x - go.transform.position.x, bounds.min.y - go.transform.position.y, bounds.center.z - go.transform.position.z) * scale;
                    }
                }
                else
                {
                    // A missing authored export is an explicit visual error, never silently accepted art.
                    var error = GameObject.CreatePrimitive(PrimitiveType.Cube); error.name = "MISSING ASSET " + asset; error.transform.SetParent(go.transform, false); error.transform.localScale = Vector3.one * .2f; error.GetComponent<Renderer>().sharedMaterial = Material("missing", Color.magenta);
                }
                entities[id] = go; entityAssets[id] = asset;
            }
            go.transform.localScale = Vector3.one * width;
            if (move) go.transform.position = position;
            return go;
        }

        private void AnimateCats()
        {
            foreach (var c in Game.Selected.Cats)
            {
                if (!c.Alive || !entities.TryGetValue("cat:" + c.Id, out var go)) continue;
                var delta = Position(c) - go.transform.position;
                bool walking = delta.sqrMagnitude > .0004f;
                go.transform.position = Vector3.Lerp(go.transform.position, Position(c), 1 - Mathf.Exp(-Time.unscaledDeltaTime * 14));
                if (walking) go.transform.rotation = Quaternion.Slerp(go.transform.rotation, Quaternion.LookRotation(delta.normalized, Vector3.up), Time.unscaledDeltaTime * 10);
                var motion = go.GetComponent<CatMotion>(); if (motion != null) motion.Animate(walking, c.Goal);
                if (entities.TryGetValue("cargo:" + c.Id, out var cargo)) cargo.transform.position = go.transform.position + Vector3.up * .6f;
            }
        }

        private void UpdateSelection()
        {
            Vector3 p; float w = 1, d = 1;
            if (SelectedCat != null) { p = Position(SelectedCat); w = .85f; d = .85f; }
            else if (SelectedBuilding != null) { var b = SelectedBuilding; p = At(b.Position) + new Vector3((b.Width - 1) * .5f, 0, (b.Depth - 1) * .5f); w = b.Width; d = b.Depth; }
            else if (Game.UI != null && Game.UI.HasPlacement) p = At(CursorTile);
            else { selection.enabled = false; return; }
            selection.enabled = true; p.y = .08f;
            selection.SetPositions(new[] { p + new Vector3(-w / 2, 0, -d / 2), p + new Vector3(-w / 2, 0, d / 2), p + new Vector3(w / 2, 0, d / 2), p + new Vector3(w / 2, 0, -d / 2) });
        }

        private Material Material(string key, Color color, bool unlit = false)
        {
            if (materials.TryGetValue(key, out var found)) return found;
            var template = Resources.Load<Material>(unlit ? "ForestSelection" : "ForestSurface");
            if (template == null) throw new InvalidOperationException("Required forest material missing; run forest_setup.");
            var m = new Material(template) { name = key, color = color, enableInstancing = true };
            if (!unlit) { m.SetFloat("_Glossiness", .05f); m.SetFloat("_Metallic", 0); }
            materials[key] = m; return m;
        }
        private static void Tint(GameObject go, Material material) { foreach (var r in go.GetComponentsInChildren<Renderer>()) r.sharedMaterial = material; }
        private void Coat(GameObject go, string id)
        {
            uint hash = 2166136261; foreach (char c in id) hash = unchecked((hash ^ c) * 16777619);
            int palette = (int)(hash % 4); if (palette == 0) return;
            Color[] fur = { new Color(.82f, .60f, .38f), new Color(.42f, .46f, .47f), new Color(.29f, .25f, .22f), new Color(.78f, .74f, .64f) };
            foreach (var renderer in go.GetComponentsInChildren<Renderer>())
            {
                var originals = renderer.sharedMaterials;
                for (int i = 0; i < originals.Length; i++)
                {
                    var source = originals[i]; if (source == null || source.name.IndexOf("fur", StringComparison.OrdinalIgnoreCase) < 0) continue;
                    string key = "coat:" + palette + ":" + source.name;
                    if (!materials.TryGetValue(key, out var replacement))
                    {
                        var color = source.name.Contains("light") ? Color.Lerp(fur[palette], new Color(.94f, .88f, .72f), .8f) : source.name.Contains("dark") ? fur[palette] * .6f : fur[palette];
                        replacement = new Material(source) { name = key, color = color, enableInstancing = true }; materials[key] = replacement;
                    }
                    originals[i] = replacement;
                }
                renderer.sharedMaterials = originals;
            }
        }
        private void ClearWorld() { foreach (var go in entities.Values) Destroy(go); entities.Clear(); entityAssets.Clear(); terrainSignature = -1; SelectedCatId = ""; SelectedBuildingId = ""; }
        private static Vector3 At(Int2 p) => new Vector3(p.X, 0, p.Z);
        private static Vector3 Position(Cat c) => new Vector3((float)c.X, 0, (float)c.Z);
        private void OnDestroy() { if (worldRoot != null) Destroy(worldRoot.gameObject); if (groundRoot != null) { foreach (var filter in groundRoot.GetComponentsInChildren<MeshFilter>()) Destroy(filter.sharedMesh); Destroy(groundRoot.gameObject); } if (selection != null) Destroy(selection.gameObject); if (sun != null) Destroy(sun.gameObject); if (ownsCamera && Camera != null) Destroy(Camera.gameObject); foreach (var material in materials.Values) Destroy(material); }
    }

    public sealed class CatMotion : MonoBehaviour
    {
        public string CatId = "";
        private Transform[] parts;
        private Quaternion[] rest;
        private Vector3[] positions;
        private float phase;
        public void Animate(bool walking, string goal)
        {
            if (parts == null)
            {
                parts = GetComponentsInChildren<Transform>().Where(t => t.name.StartsWith("Paw") || t.name == "Tail" || t.name == "Head" || t.name == "Body").ToArray();
                rest = parts.Select(t => t.localRotation).ToArray(); positions = parts.Select(t => t.localPosition).ToArray();
                uint hash = 2166136261; foreach (char c in CatId) hash = unchecked((hash ^ c) * 16777619);
                phase = hash % 628 / 100f;
            }
            float time = Time.time;
            bool sleeping = goal == "need_sleep" && !walking;
            bool working = !walking && (goal.Contains("work") || goal.Contains("harvest") || goal.Contains("produce"));
            for (int i = 0; i < parts.Length; i++)
            {
                bool opposite = parts[i].name == "PawFrontRight" || parts[i].name == "PawBackLeft";
                float angle = parts[i].name.StartsWith("Paw") ? (walking ? Mathf.Sin(time * 11 + phase + (opposite ? Mathf.PI : 0)) * 26 : working && parts[i].name.StartsWith("PawFront") ? Mathf.Sin(time * 5 + phase) * 12 : sleeping ? -28 : 0)
                    : parts[i].name == "Tail" ? Mathf.Sin(time * 1.6f + phase) * (sleeping ? 3 : 12)
                    : parts[i].name == "Head" ? sleeping ? 22 : working ? 10 + Mathf.Sin(time * 5 + phase) * 6 : Mathf.Sin(time * .8f + phase) * 4 : 0;
                float glance = parts[i].name == "Head" && !walking && !sleeping && !working ? Mathf.Sin(time * .45f + phase) * 12 : 0;
                parts[i].localRotation = rest[i] * Quaternion.Euler(angle, glance, 0);
                parts[i].localPosition = positions[i] + (parts[i].name == "Body" ? Vector3.up * (walking ? Mathf.Sin(time * 22 + phase) * .012f : Mathf.Sin(time * 2 + phase) * .007f) : Vector3.zero);
            }
        }
    }
}
