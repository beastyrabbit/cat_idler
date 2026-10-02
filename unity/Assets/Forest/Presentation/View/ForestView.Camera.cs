using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Management (orthographic, rotatable, smoothed) and direct-control (follow) cameras, plus the
    /// intro swoop. Other partials move the management camera by writing its targets
    /// (<c>focus</c>, <c>zoom</c>); the rendered camera eases toward them every frame, so selection,
    /// level and dungeon jumps glide instead of teleporting. Drags and key pans move it 1:1.
    /// </summary>
    public sealed partial class ForestView
    {
        // Tuning. Zoom is the orthographic half-height in world units (1 unit = 1 tile).
        internal const float ManagementStandoff = 25;
        internal const float DefaultYaw = 40, DefaultPitch = 50, CloseUpPitch = 40;
        /// <summary>At 1920x1080 a cat (0.85 units long) reads about 50 px tall on screen.</summary>
        internal const float DefaultZoom = 8.5f;
        internal const float MinZoom = 3.5f, MaxZoom = 22;
        internal const float ZoomSmoothSeconds = .12f, FocusSmoothSeconds = .25f, YawSmoothSeconds = .3f, PitchSmoothSeconds = .12f;
        /// <summary>Tiles the framed point may leave the known area by.</summary>
        internal const float FocusMargin = 4;
        internal const float IntroSeconds = 3;
        private const float IntroStartPitch = 82, IntroStartYawOffset = -50;
        private const float WheelZoomPerStep = .12f, KeyPanSpeed = 1.1f;
        private static readonly Vector3 DirectOffset = new Vector3(0, 2.5f, -3.8f);

        // Targets: where the management camera is heading. Other partials write focus and zoom.
        private Vector3 focus, managementFocus;
        private float zoom = DefaultZoom, managementZoom = DefaultZoom, managementYaw = DefaultYaw;
        /// <summary>Yaw of the direct-control camera around its cat (read by the dungeon walls).</summary>
        private float yaw;
        // The rendered management camera.
        private Vector3 shownFocus, focusVelocity;
        private float shownZoom = DefaultZoom, zoomVelocity, shownYaw = DefaultYaw, yawVelocity, shownPitch = DefaultPitch, pitchVelocity;
        private bool cameraPlaced, wasDirect, ownsCamera;
        private int shownLevel = int.MinValue;
        // A wheel zoom keeps the world point under the cursor fixed while the zoom eases.
        private bool zoomAnchored;
        private Vector2 zoomAnchorScreen;
        private Vector3 zoomAnchorWorld;
        // FocusSelected keeps framing this cat until a manual pan.
        private string followCatId = "";
        private Vector3 lastControlledPosition;
        private int lastControlledLevel;
        private bool hasControlledPosition;
        // Known-tile bounds of the rendered level, cached on the known count.
        private string boundsVillage = "";
        private int boundsLevel = int.MinValue, boundsCount = -1;
        private bool hasBounds;
        private Vector4 knownBounds;
        // Intro swoop.
        private const double FreshColonySeconds = 5;
        private bool introActive, introOffered;
        private float introElapsed, introZoom, introYaw;
        private Vector3 introFromFocus, introFocus;

        public float ManagementZoom => DirectControl ? managementZoom : zoom;
        /// <summary>Orthographic half-height the management camera is heading to, also while it is parked in direct control.</summary>
        internal float Zoom => zoom;
        /// <summary>Yaw of the active camera in degrees: the management view's yaw, or the direct-control yaw around the cat.</summary>
        internal float CameraYaw => DirectControl ? yaw : shownYaw;
        /// <summary>Pitch of the management camera in degrees (50 by default, easing to 40 when zoomed in).</summary>
        internal float CameraPitch => shownPitch;
        /// <summary>Distance from the camera to the point it frames.</summary>
        internal float CameraStandoff => DirectControl ? DirectOffset.magnitude : ManagementStandoff;
        /// <summary>The intro swoop is running.</summary>
        public bool IntroPlaying => introActive;
        /// <summary>The management camera keeps the cat with this id framed ("" when it follows nobody).</summary>
        internal string FollowedCatId => followCatId;
        /// <summary>The point the rendered management camera frames.</summary>
        internal Vector3 CameraFocus => shownFocus;
        /// <summary>The point the management camera is heading to.</summary>
        internal Vector3 CameraTarget => focus;
        /// <summary>The yaw the management camera is turning to, in degrees.</summary>
        internal float CameraTargetYaw => managementYaw;

        private void CreateCamera()
        {
            Camera = UnityEngine.Camera.main;
            if (Camera == null) { var go = new GameObject("Forest camera"); Camera = go.AddComponent<Camera>(); go.tag = "MainCamera"; ownsCamera = true; }
            Camera.clearFlags = CameraClearFlags.SolidColor;
            Camera.backgroundColor = new Color(.11f, .18f, .17f);
            Camera.nearClipPlane = .1f; Camera.farClipPlane = 240;
            Camera.allowHDR = false; Camera.allowMSAA = true;
        }

        /// <summary>Zooms the management camera by normalized wheel steps (positive zooms in) around the framed point.</summary>
        public void ZoomBy(float steps)
        {
            if (DirectControl || float.IsNaN(steps) || float.IsInfinity(steps)) return;
            zoom = Mathf.Clamp(zoom * Mathf.Exp(Mathf.Clamp(-steps * WheelZoomPerStep, -20, 20)), MinZoom, MaxZoom);
            zoomAnchored = false;
            EndIntro();
        }

        /// <summary>Zooms by wheel steps while the world point under <paramref name="screen"/> stays under it.</summary>
        internal void ZoomAround(float steps, Vector2 screen)
        {
            if (DirectControl || float.IsNaN(steps) || float.IsInfinity(steps)) return;
            EndIntro();
            ApplyManagementCamera();
            var anchor = GroundPoint(screen);
            ZoomBy(steps);
            zoomAnchored = true; zoomAnchorScreen = screen; zoomAnchorWorld = anchor;
        }

        /// <summary>Frames the selected village at the default zoom and rebuilds its entities (also after a world switch).</summary>
        public void FocusVillage()
        {
            viewedLevel = 0;
            var v = Game.Selected; if (v == null) return;
            var center = new Vector3(v.Center.X, 0, v.Center.Z);
            if (DirectControl) { managementFocus = center; managementZoom = DefaultZoom; return; }
            focus = center; zoom = DefaultZoom; villageId = ""; followCatId = "";
            EndIntro();
        }

        /// <summary>Home: glides back to the village centre on the surface, keeping the selection and the rotation.</summary>
        public void GoHome()
        {
            var v = Game.Selected; if (v == null || DirectControl) return;
            viewedLevel = 0; nextReconcile = 0; followCatId = "";
            focus = At(v.Center); zoom = DefaultZoom;
            EndIntro();
        }

        /// <summary>Turns the management view by 90° steps (positive turns the view clockwise, seen from above).</summary>
        public void RotateView(int quarterTurns)
        {
            if (DirectControl || quarterTurns == 0) return;
            managementYaw = Mathf.Repeat(managementYaw + 90 * quarterTurns, 360);
            EndIntro();
        }

        /// <summary>Frames the selected cat (and keeps following it until a manual pan) or the selected building.</summary>
        public bool FocusSelected()
        {
            if (!WorldReady || DirectControl) return false;
            var cat = SelectedCat;
            if (cat != null && cat.Alive)
            {
                if (cat.Position.Level != viewedLevel) { viewedLevel = cat.Position.Level; nextReconcile = 0; }
                focus = Position(cat); followCatId = cat.Id;
                EndIntro();
                return true;
            }
            var b = SelectedBuilding;
            if (b == null) return false;
            if (b.Position.Level != viewedLevel) { viewedLevel = b.Position.Level; nextReconcile = 0; }
            focus = At(b.Position) + new Vector3((b.Width - 1) * .5f, 0, (b.Depth - 1) * .5f); followCatId = "";
            EndIntro();
            return true;
        }

        /// <summary>
        /// The 3 s opening swoop from canopy height to the default framing of the colony leader (or the
        /// village centre). Any input skips it; with reduced motion the camera starts framed.
        /// </summary>
        public void PlayIntro()
        {
            if (!WorldReady || DirectControl) return;
            var v = Game.Selected;
            var leader = v.Cats.Find(c => c.Alive && c.Id == v.LeaderId && c.Position.Level == 0);
            viewedLevel = 0; followCatId = ""; zoomAnchored = false;
            focus = ClampFocus(leader != null ? Position(leader) : At(v.Center));
            zoom = DefaultZoom; managementYaw = DefaultYaw;
            if (GameSettings.ReducedMotion) { introActive = false; SnapManagementCamera(); ApplyManagementCamera(); return; }
            introActive = true; introElapsed = 0;
            introFromFocus = ClampFocus(At(v.Center)); introFocus = focus; introZoom = zoom; introYaw = managementYaw;
            shownLevel = RenderedLevel;
            AdvanceIntro(0);
            ApplyManagementCamera();
        }

        /// <summary>
        /// The swoop plays once per run, on the first frame of a freshly founded local colony. Batch runs
        /// (tests) start framed; they call <see cref="PlayIntro"/> themselves.
        /// </summary>
        private bool IntroWanted()
        {
            if (introOffered) return false;
            introOffered = true;
            return !Application.isBatchMode && !Game.IsRemote && Game.CurrentWorld.TimeSeconds < FreshColonySeconds;
        }

        /// <summary>Stops the intro; the camera eases from wherever the swoop was to its targets.</summary>
        private void EndIntro()
        {
            if (!introActive) return;
            introActive = false;
            focusVelocity = Vector3.zero; zoomVelocity = yawVelocity = pitchVelocity = 0;
        }

        /// <summary>Moves the management camera and its target together (key, drag and trackpad pans).</summary>
        private void PanBy(Vector3 delta)
        {
            delta.y = 0;
            if (delta.sqrMagnitude < 1e-10f) return;
            EndIntro();
            followCatId = ""; zoomAnchored = false;
            focus = ClampFocus(focus + delta);
            shownFocus = ClampFocus(shownFocus + delta);
            ApplyManagementCamera();
        }

        /// <summary>Puts the rendered management camera on its targets at once.</summary>
        private void SnapManagementCamera()
        {
            focus = ClampFocus(focus);
            shownFocus = focus; shownZoom = zoom; shownYaw = managementYaw; shownPitch = PitchFor(zoom);
            focusVelocity = Vector3.zero; zoomVelocity = yawVelocity = pitchVelocity = 0;
            shownLevel = RenderedLevel; cameraPlaced = true;
        }

        private static float PitchFor(float zoomLevel)
        {
            float t = Mathf.InverseLerp(MinZoom, DefaultZoom, zoomLevel);
            return Mathf.Lerp(CloseUpPitch, DefaultPitch, t * t * (3 - 2 * t));
        }

        /// <summary>Places the camera at the rendered management pose; nothing here jumps toward the targets.</summary>
        private void ApplyManagementCamera()
        {
            Camera.orthographic = true; Camera.orthographicSize = shownZoom;
            Camera.transform.rotation = Quaternion.Euler(shownPitch, shownYaw, 0);
            Camera.transform.position = shownFocus - Camera.transform.forward * ManagementStandoff;
        }

        internal void UpdateCamera()
        {
            var cat = ControlledCat;
            float dt = Mathf.Min(Time.unscaledDeltaTime, .1f);
            if (cat != null)
            {
                if (!wasDirect) { managementFocus = focus; managementZoom = zoom; EndIntro(); followCatId = ""; zoomAnchored = false; }
                Camera.orthographic = false; Camera.fieldOfView = 58;
                lastControlledPosition = Position(cat); lastControlledLevel = cat.Position.Level; hasControlledPosition = true;
                var target = lastControlledPosition + Vector3.up * .55f;
                var offset = Quaternion.Euler(0, yaw, 0) * DirectOffset;
                Camera.transform.position = Vector3.Lerp(Camera.transform.position, target + offset, 1 - Mathf.Exp(-Time.unscaledDeltaTime * 8));
                Camera.transform.LookAt(target);
            }
            else
            {
                if (wasDirect)
                {
                    // Leaving control frames the cat where it now stands instead of jumping back to where control began.
                    zoom = managementZoom;
                    if (hasControlledPosition) { viewedLevel = lastControlledLevel; focus = lastControlledPosition; nextReconcile = 0; }
                    else focus = managementFocus;
                    SnapManagementCamera();
                }
                StepManagementCamera(dt);
            }
            ApplyFogForCamera(cat != null);
            wasDirect = cat != null;
        }

        private void StepManagementCamera(float dt)
        {
            FollowCat();
            focus = ClampFocus(focus);
            if (!cameraPlaced || RenderedLevel != shownLevel) SnapManagementCamera();
            if (introActive)
            {
                if (focus != introFocus || !Mathf.Approximately(zoom, introZoom) || !Mathf.Approximately(managementYaw, introYaw)) EndIntro();
                else AdvanceIntro(dt);
            }
            if (!introActive)
            {
                shownZoom = Mathf.SmoothDamp(shownZoom, zoom, ref zoomVelocity, ZoomSmoothSeconds, Mathf.Infinity, dt);
                shownYaw = Mathf.SmoothDampAngle(shownYaw, managementYaw, ref yawVelocity, YawSmoothSeconds, Mathf.Infinity, dt);
                shownPitch = Mathf.SmoothDamp(shownPitch, PitchFor(shownZoom), ref pitchVelocity, PitchSmoothSeconds, Mathf.Infinity, dt);
                shownFocus = Vector3.SmoothDamp(shownFocus, focus, ref focusVelocity, FocusSmoothSeconds, Mathf.Infinity, dt);
            }
            ApplyManagementCamera();
            if (zoomAnchored)
            {
                var drift = zoomAnchorWorld - PlanePoint(zoomAnchorScreen, zoomAnchorWorld.y); drift.y = 0;
                focus = ClampFocus(focus + drift); shownFocus = ClampFocus(shownFocus + drift);
                ApplyManagementCamera();
                if (Mathf.Abs(shownZoom - zoom) < zoom * 1e-3f) zoomAnchored = false;
            }
        }

        private void AdvanceIntro(float dt)
        {
            introElapsed += dt;
            float t = Mathf.Clamp01(introElapsed / IntroSeconds);
            float glide = t * t * (3 - 2 * t);
            float drop = 1 - (1 - t) * (1 - t) * (1 - t);
            shownFocus = Vector3.Lerp(introFromFocus, introFocus, glide);
            shownZoom = Mathf.Lerp(MaxZoom, introZoom, drop);
            shownPitch = Mathf.Lerp(IntroStartPitch, PitchFor(introZoom), glide);
            shownYaw = introYaw + IntroStartYawOffset * (1 - glide);
            if (t >= 1) { introActive = false; focusVelocity = Vector3.zero; zoomVelocity = yawVelocity = pitchVelocity = 0; }
        }

        private void FollowCat()
        {
            if (followCatId == "") return;
            var cat = Game.Selected.Cats.Find(c => c.Id == followCatId);
            if (cat == null || !cat.Alive || SelectedCatId != followCatId) { followCatId = ""; return; }
            if (cat.Position.Level != viewedLevel) { viewedLevel = cat.Position.Level; nextReconcile = 0; }
            focus = Position(cat);
        }

        /// <summary>Keeps the framed point within the known tiles of the rendered level plus a margin.</summary>
        private Vector3 ClampFocus(Vector3 point)
        {
            var v = Game.Selected;
            if (v == null) return point;
            int level = RenderedLevel;
            if (boundsVillage != v.Id || boundsLevel != level || boundsCount != v.Known.Count)
            {
                boundsVillage = v.Id; boundsLevel = level; boundsCount = v.Known.Count;
                var cells = v.Known.Where(p => p.Level == level).ToList();
                hasBounds = cells.Count > 0;
                if (hasBounds) knownBounds = new Vector4(cells.Min(p => p.X), cells.Min(p => p.Z), cells.Max(p => p.X), cells.Max(p => p.Z));
            }
            if (!hasBounds) return point;
            point.x = Mathf.Clamp(point.x, knownBounds.x - FocusMargin, knownBounds.z + FocusMargin);
            point.z = Mathf.Clamp(point.z, knownBounds.y - FocusMargin, knownBounds.w + FocusMargin);
            return point;
        }

        /// <summary>Where a screen ray meets the horizontal plane at <paramref name="height"/> (stable under hills and water).</summary>
        private Vector3 PlanePoint(Vector2 screen, float height)
        {
            var ray = Camera.ScreenPointToRay(screen);
            return new Plane(Vector3.up, new Vector3(0, height, 0)).Raycast(ray, out float distance) ? ray.GetPoint(distance) : shownFocus;
        }
    }
}
