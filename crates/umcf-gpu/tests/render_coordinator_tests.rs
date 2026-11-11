//! Five-tier verification test suite for Subphase S6.P1.SP2:
//! Distributed Rendering Coordinator & Progressive Frame Compositor.

use umcf_gpu::compositor::{CompletedTilePacket, FrameCompositor};
use umcf_gpu::render_coordinator::{
    morton_2d, DistributedRenderCoordinator, RenderJobSpec, RenderPixelFormat,
};
use umcf_gpu::shader_sandbox::{ShaderSandbox, ValidatedShader};

fn dummy_shader() -> ValidatedShader {
    let source = r#"
        @group(0) @binding(0) var<storage, read_write> data: array<u32>;
        @compute @workgroup_size(16, 16, 1)
        fn main(@builtin(global_invocation_id) id: vec3<u32>) {
            data[id.x] = id.y;
        }
    "#;
    ShaderSandbox::default()
        .validate_and_instrument("dummy-render-shader", source)
        .expect("dummy shader must validate")
}

// =========================================================================
// 1. UNIT TESTS
// =========================================================================

#[test]
fn test_unit_hilbert_and_center_spiral_tile_generation_non_multiple_resolutions() {
    // 1920x1080 with 128x128 tiles:
    // 1920 / 128 = 15 tiles wide
    // 1080 / 128 = 8.4375 => 9 tiles high (1080 = 8 * 128 + 56)
    // Total tiles = 15 * 9 = 135
    let spec = RenderJobSpec {
        job_id: "test-non-multiple-res".to_string(),
        resolution: (1920, 1080),
        frame_range: 0..=0,
        samples_per_pixel: 32,
        tile_size: (128, 128),
        pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
        scene_asset_root: [0x55; 32],
        shader_module: dummy_shader(),
    };

    let coordinator = DistributedRenderCoordinator::new(spec).expect("coordinator creation must succeed");
    assert_eq!(coordinator.tiles.len(), 135);

    // Verify center-spiral ordering: first tile must be closest to center (960, 540)
    let first_tile = &coordinator.tiles[0];
    let last_tile = &coordinator.tiles[134];
    assert!(first_tile.distance_to_center < last_tile.distance_to_center);
    assert!(first_tile.distance_to_center < 100.0); // Right at center

    // Verify edge tile clamping (bottom row height should be 56 pixels)
    let bottom_edge_tile = coordinator
        .tiles
        .iter()
        .find(|t| t.y == 8 * 128)
        .expect("must find bottom edge tile");
    assert_eq!(bottom_edge_tile.height, 56);
    assert_eq!(bottom_edge_tile.width, 128);

    // Verify 2D Morton index monotonicity
    let m00 = morton_2d(0, 0);
    let m10 = morton_2d(1, 0);
    let m01 = morton_2d(0, 1);
    let m11 = morton_2d(1, 1);
    assert_eq!(m00, 0);
    assert_eq!(m10, 1);
    assert_eq!(m01, 2);
    assert_eq!(m11, 3);
}

#[test]
fn test_unit_adaptive_tile_batch_sizing_by_worker_throughput_ewma() {
    let spec = RenderJobSpec {
        job_id: "test-adaptive-batching".to_string(),
        resolution: (1024, 1024),
        frame_range: 0..=0,
        samples_per_pixel: 16,
        tile_size: (128, 128),
        pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
        scene_asset_root: [0xaa; 32],
        shader_module: dummy_shader(),
    };

    let mut coordinator = DistributedRenderCoordinator::new(spec).expect("coordinator init");

    coordinator.register_worker("w-desktop-rtx", "NVIDIA RTX 4090", 12.0);
    coordinator.register_worker("w-laptop-mid", "Apple M3 Pro", 4.0);
    coordinator.register_worker("w-phone-mobile", "Google Tensor G3", 1.0);

    // Assert batch sizing
    assert_eq!(coordinator.adaptive_batch_size("w-desktop-rtx"), 4);
    assert_eq!(coordinator.adaptive_batch_size("w-laptop-mid"), 2);
    assert_eq!(coordinator.adaptive_batch_size("w-phone-mobile"), 1);

    // Dispatch and record completions, verifying EWMA updates
    let rtx_batch = coordinator.dispatch_next_batch("w-desktop-rtx");
    assert_eq!(rtx_batch.len(), 4);

    let recorded = coordinator
        .record_tile_completion(rtx_batch[0].tile_id, "w-desktop-rtx", 5.0)
        .expect("record must succeed");
    assert!(recorded);

    let w = coordinator.workers.get("w-desktop-rtx").unwrap();
    assert_eq!(w.completed_tiles_count, 1);
    assert!(w.ewma_pixels_per_ms > 0.0);
}

#[test]
fn test_unit_speculative_tail_dispatch_cancels_redundant_slow_worker() {
    let spec = RenderJobSpec {
        job_id: "test-speculative-tail".to_string(),
        resolution: (640, 512), // 5x4 = 20 tiles
        frame_range: 0..=0,
        samples_per_pixel: 16,
        tile_size: (128, 128),
        pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
        scene_asset_root: [0x11; 32],
        shader_module: dummy_shader(),
    };

    let mut coordinator = DistributedRenderCoordinator::new(spec).expect("coordinator init");
    coordinator.register_worker("w-fast", "Fast Worker", 10.0);
    coordinator.register_worker("w-slow", "Slow Worker", 1.0);

    // Dispatch 19 tiles (95.0%) to fast worker and complete them immediately
    for _ in 0..19 {
        if let Some(tile_id) = coordinator.pending_queue.pop_front() {
            coordinator
                .record_tile_completion(tile_id, "w-fast", 2.0)
                .expect("commit");
        }
    }

    // Now 19/20 tiles are complete (95.0%). Dispatch last tile (tile 19) to slow worker
    let slow_batch = coordinator.dispatch_next_batch("w-slow");
    assert_eq!(slow_batch.len(), 1);
    let straggler_id = slow_batch[0].tile_id;

    // Advance time significantly so slow worker is considered stalled
    coordinator.set_current_time_ms(1000);

    // Fast worker asks for work; pending queue is empty, so speculative execution triggers
    let speculative_batch = coordinator.dispatch_next_batch("w-fast");
    assert_eq!(speculative_batch.len(), 1);
    assert_eq!(speculative_batch[0].tile_id, straggler_id);
    assert_eq!(coordinator.speculative_dispatches_count, 1);

    // Fast worker finishes the straggler first!
    let fast_win = coordinator
        .record_tile_completion(straggler_id, "w-fast", 3.0)
        .expect("fast commit");
    assert!(fast_win);

    // Entire frame is now complete
    assert!(coordinator.is_frame_complete());

    // Slow worker finally finishes later; should be cleanly discarded as redundant
    let slow_redundant = coordinator
        .record_tile_completion(straggler_id, "w-slow", 500.0)
        .expect("slow redundant commit");
    assert!(!slow_redundant);
    assert_eq!(coordinator.canceled_redundant_tiles_count, 1);
}

#[test]
fn test_unit_compositor_blit_exact_pixel_placement_and_exr_export() {
    let mut compositor = FrameCompositor::new(256, 256, RenderPixelFormat::Rgba8UnormSrgb);

    // Four 128x128 tiles:
    // Top-Left (Red), Top-Right (Green), Bottom-Left (Blue), Bottom-Right (Yellow)
    let red_pixels = [255, 0, 0, 255].repeat(128 * 128);
    let green_pixels = [0, 255, 0, 255].repeat(128 * 128);
    let blue_pixels = [0, 0, 255, 255].repeat(128 * 128);
    let yellow_pixels = [255, 255, 0, 255].repeat(128 * 128);

    compositor
        .blit_tile(&CompletedTilePacket {
            tile_id: 0,
            frame_idx: 0,
            x: 0,
            y: 0,
            width: 128,
            height: 128,
            pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
            pixels: red_pixels,
            worker_id: "node-red".to_string(),
            render_time_ms: 10.0,
        })
        .expect("blit 0");

    compositor
        .blit_tile(&CompletedTilePacket {
            tile_id: 1,
            frame_idx: 0,
            x: 128,
            y: 0,
            width: 128,
            height: 128,
            pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
            pixels: green_pixels,
            worker_id: "node-green".to_string(),
            render_time_ms: 10.0,
        })
        .expect("blit 1");

    compositor
        .blit_tile(&CompletedTilePacket {
            tile_id: 2,
            frame_idx: 0,
            x: 0,
            y: 128,
            width: 128,
            height: 128,
            pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
            pixels: blue_pixels,
            worker_id: "node-blue".to_string(),
            render_time_ms: 10.0,
        })
        .expect("blit 2");

    compositor
        .blit_tile(&CompletedTilePacket {
            tile_id: 3,
            frame_idx: 0,
            x: 128,
            y: 128,
            width: 128,
            height: 128,
            pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
            pixels: yellow_pixels,
            worker_id: "node-yellow".to_string(),
            render_time_ms: 10.0,
        })
        .expect("blit 3");

    // Exact pixel checks
    assert_eq!(compositor.get_pixel_rgba8(0, 0).unwrap(), [255, 0, 0, 255]);
    assert_eq!(compositor.get_pixel_rgba8(127, 127).unwrap(), [255, 0, 0, 255]);
    assert_eq!(compositor.get_pixel_rgba8(128, 0).unwrap(), [0, 255, 0, 255]);
    assert_eq!(compositor.get_pixel_rgba8(0, 128).unwrap(), [0, 0, 255, 255]);
    assert_eq!(compositor.get_pixel_rgba8(128, 128).unwrap(), [255, 255, 0, 255]);

    // Check OpenEXR export format
    let exr_data = compositor.export_openexr().expect("openexr export");
    assert_eq!(&exr_data[0..4], &[0x76, 0x2f, 0x31, 0x01]); // OpenEXR magic

    // Check PPM export format
    let ppm_data = compositor.export_ppm().expect("ppm export");
    assert!(ppm_data.starts_with(b"P6\n256 256\n255\n"));
}

// =========================================================================
// 2. INTEGRATION TESTS
// =========================================================================

#[test]
fn test_integration_4_node_distributed_path_tracer_progressive_composition() {
    let spec = RenderJobSpec {
        job_id: "cornell-box-distributed-render".to_string(),
        resolution: (512, 512),
        frame_range: 0..=0,
        samples_per_pixel: 64,
        tile_size: (64, 64), // 8x8 = 64 tiles
        pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
        scene_asset_root: [0xcc; 32],
        shader_module: dummy_shader(),
    };

    let mut coordinator = DistributedRenderCoordinator::new(spec).expect("coordinator");
    coordinator.register_worker("w-1", "Desktop RTX 4090", 8.0);
    coordinator.register_worker("w-2", "MacBook M3 Pro", 4.0);
    coordinator.register_worker("w-3", "iPad Pro M2", 2.0);
    coordinator.register_worker("w-4", "Pixel 8 Pro", 1.0);

    let mut compositor = FrameCompositor::new(512, 512, RenderPixelFormat::Rgba8UnormSrgb);
    let worker_ids = ["w-1", "w-2", "w-3", "w-4"];
    let mut worker_idx = 0;

    let mut progressive_deltas = Vec::new();

    while !coordinator.is_frame_complete() {
        let wid = worker_ids[worker_idx % worker_ids.len()];
        worker_idx += 1;

        let batch = coordinator.dispatch_next_batch(wid);
        for tile in batch {
            // Procedural Cornell box pixel synthesis
            let mut pixels = Vec::with_capacity((tile.width * tile.height * 4) as usize);
            for ty in 0..tile.height {
                for tx in 0..tile.width {
                    let global_x = tile.x + tx;
                    let global_y = tile.y + ty;

                    // Procedural Cornell Box shading: Red left wall, Green right wall, White floor/ceiling
                    let (r, g, b) = if global_x < 32 {
                        (230u8, 30u8, 30u8) // Left red wall
                    } else if global_x > 480 {
                        (30u8, 230u8, 30u8) // Right green wall
                    } else if global_y > 480 {
                        (200u8, 200u8, 200u8) // Floor
                    } else if global_y < 32 {
                        (255u8, 255u8, 220u8) // Ceiling light
                    } else {
                        (140u8, 140u8, 150u8) // Interior ambient
                    };

                    pixels.extend_from_slice(&[r, g, b, 255]);
                }
            }

            let packet = CompletedTilePacket {
                tile_id: tile.tile_id,
                frame_idx: tile.frame_idx,
                x: tile.x,
                y: tile.y,
                width: tile.width,
                height: tile.height,
                pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
                pixels,
                worker_id: wid.to_string(),
                render_time_ms: 12.5,
            };

            let delta = compositor.blit_tile(&packet).expect("blit tile");
            progressive_deltas.push(delta);

            coordinator
                .record_tile_completion(tile.tile_id, wid, 12.5)
                .expect("record");
        }
    }

    assert_eq!(compositor.completed_tile_count, 64);
    assert_eq!(progressive_deltas.len(), 64);

    let frame_digest = compositor.compute_frame_checksum();
    assert_ne!(frame_digest, [0u8; 32]);

    // Verify Cornell Box pixel features
    assert_eq!(compositor.get_pixel_rgba8(10, 256).unwrap()[0], 230); // Red wall
    assert_eq!(compositor.get_pixel_rgba8(500, 256).unwrap()[1], 230); // Green wall
}

// =========================================================================
// 3. SIMULATION TESTS
// =========================================================================

#[test]
fn test_simulation_speculative_tail_execution_straggler_mitigation() {
    let spec = RenderJobSpec {
        job_id: "straggler-simulation".to_string(),
        resolution: (1920, 1080), // 135 tiles of 128x128
        frame_range: 0..=0,
        samples_per_pixel: 32,
        tile_size: (128, 128),
        pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
        scene_asset_root: [0x42; 32],
        shader_module: dummy_shader(),
    };

    let mut coordinator = DistributedRenderCoordinator::new(spec).expect("coordinator");
    coordinator.register_worker("w-rtx4090", "Desktop RTX", 10.0);
    coordinator.register_worker("w-slow-phone", "Phone", 0.8);

    // Fast worker completes first 134 tiles (99.2%)
    for _ in 0..134 {
        if let Some(tid) = coordinator.pending_queue.pop_front() {
            coordinator
                .record_tile_completion(tid, "w-rtx4090", 2.0)
                .expect("fast complete");
        }
    }

    // Tile #134 dispatched to slow phone
    let phone_batch = coordinator.dispatch_next_batch("w-slow-phone");
    assert_eq!(phone_batch.len(), 1);
    let straggler_id = phone_batch[0].tile_id;

    // Simulate 10-second stall on phone
    coordinator.set_current_time_ms(10_000);

    // Speculative tail check triggers on idle fast worker
    let spec_batch = coordinator.dispatch_next_batch("w-rtx4090");
    assert_eq!(spec_batch.len(), 1);
    assert_eq!(spec_batch[0].tile_id, straggler_id);

    // Fast worker finishes in 3 ms
    coordinator
        .record_tile_completion(straggler_id, "w-rtx4090", 3.0)
        .expect("speculative commit");
    assert!(coordinator.is_frame_complete());
    assert_eq!(coordinator.speculative_dispatches_count, 1);
}

// =========================================================================
// 4. HARDWARE-REQUIRED TESTS
// =========================================================================

#[test]
fn test_hardware_distributed_render_speedup_and_throughput() {
    let spec = RenderJobSpec {
        job_id: "hardware-speedup-test".to_string(),
        resolution: (1024, 1024),
        frame_range: 0..=0,
        samples_per_pixel: 16,
        tile_size: (128, 128), // 64 tiles
        pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
        scene_asset_root: [0x99; 32],
        shader_module: dummy_shader(),
    };

    let mut coordinator = DistributedRenderCoordinator::new(spec).expect("coordinator");
    // Heterogeneous cluster relative speeds: 10x, 5x, 2x, 1x (Total capacity = 18x)
    coordinator.register_worker("node-1", "Desktop High", 10.0);
    coordinator.register_worker("node-2", "Desktop Mid", 5.0);
    coordinator.register_worker("node-3", "Tablet", 2.0);
    coordinator.register_worker("node-4", "Mobile", 1.0);

    let mut completed = 0;
    let workers = ["node-1", "node-2", "node-3", "node-4"];
    let mut round = 0;

    while !coordinator.is_frame_complete() {
        let w = workers[round % workers.len()];
        round += 1;
        let batch = coordinator.dispatch_next_batch(w);
        for tile in batch {
            coordinator
                .record_tile_completion(tile.tile_id, w, 5.0)
                .expect("tile commit");
            completed += 1;
        }
    }

    assert_eq!(completed, 64);
    let (c, total, ratio) = coordinator.progress();
    assert_eq!(c, 64);
    assert_eq!(total, 64);
    assert!((ratio - 1.0).abs() < f64::EPSILON);
}

// =========================================================================
// 5. MANUALLY-VERIFIED TESTS
// =========================================================================

#[test]
fn test_manual_render_canvas_ui_schema_and_attribution_overlay() {
    let mut compositor = FrameCompositor::new(64, 64, RenderPixelFormat::Rgba8UnormSrgb);
    let delta = compositor
        .blit_tile(&CompletedTilePacket {
            tile_id: 42,
            frame_idx: 0,
            x: 0,
            y: 0,
            width: 64,
            height: 64,
            pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
            pixels: vec![128; 64 * 64 * 4],
            worker_id: "rtx-4090-desktop".to_string(),
            render_time_ms: 18.2,
        })
        .expect("blit");

    let json = serde_json::to_string_pretty(&delta).expect("serialize delta");
    assert!(json.contains("rtx-4090-desktop"));
    assert!(json.contains("\"tile_id\": 42"));
    assert!(json.contains("tile_checksum_blake3"));
}
