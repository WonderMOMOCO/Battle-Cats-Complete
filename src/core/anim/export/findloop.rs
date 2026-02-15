use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};

use crate::data::global::mamodel::Model;
use crate::data::global::maanim::Animation;
use crate::core::anim::{animator, transform};
use crate::core::anim::export::state::LoopStatus;

const TIMEOUT_SECONDS: u64 = 180; // 3 Minutes

pub fn start_search(
    model: Model,
    anim: Animation,
    tolerance: i32,
    min_loop: i32,
    max_loop: Option<i32>,
    status_tx: mpsc::Sender<LoopStatus>,
    abort_flag: Arc<AtomicBool>
) {
    thread::spawn(move || {
        let start_time = Instant::now();
        // Store Matrix [f32; 9] and Opacity (f32) for every part
        let mut frame_states: Vec<Vec<([f32; 9], f32)>> = Vec::new();
        
        let mut current_frame = 0;
        
        loop {
            // Check Abort or Timeout
            if abort_flag.load(Ordering::Relaxed) {
                let _ = status_tx.send(LoopStatus::Error("Aborted".to_string()));
                break;
            }
            if start_time.elapsed().as_secs() > TIMEOUT_SECONDS {
                let _ = status_tx.send(LoopStatus::Error("Timed out (3 mins)".to_string()));
                break;
            }

            // Solve current frame
            let f = current_frame as f32;
            let parts = animator::animate(&model, &anim, f);
            let world_parts = transform::solve_hierarchy(&parts, &model);
            
            // Extract comparable state (Matrix + Opacity)
            // We compare indices 0,1,3,4 (Scale/Rot/Shear) and 6,7 (Translation X/Y)
            let mut current_state = Vec::with_capacity(world_parts.len());
            for part in &world_parts {
                current_state.push((part.matrix, part.opacity));
            }

            // Compare against history
            // We look backwards to find the first frame that matches current
            let mut found_match = None;

            for (past_frame_idx, past_state) in frame_states.iter().enumerate() {
                let loop_len = current_frame - past_frame_idx as i32;
                
                // Min Loop Check
                if loop_len < min_loop { continue; }
                
                // Max Loop Check
                if let Some(max) = max_loop {
                    if loop_len > max { continue; }
                }

                let mut diff_sum = 0.0;
                
                for (i, (c_mat, c_op)) in current_state.iter().enumerate() {
                    if let Some((p_mat, p_op)) = past_state.get(i) {
                        // Compare Matrix Elements
                        diff_sum += (c_mat[6] - p_mat[6]).abs();
                        diff_sum += (c_mat[7] - p_mat[7]).abs();

                        // We weight these higher (x100) because small changes in scale/rot are very visible
                        diff_sum += (c_mat[0] - p_mat[0]).abs() * 100.0;
                        diff_sum += (c_mat[1] - p_mat[1]).abs() * 100.0;
                        diff_sum += (c_mat[3] - p_mat[3]).abs() * 100.0;
                        diff_sum += (c_mat[4] - p_mat[4]).abs() * 100.0;

                        // Opacity, weight by 255 for "alpha byte" roughly
                        diff_sum += (c_op - p_op).abs() * 255.0;
                    }
                }

                if diff_sum <= tolerance as f32 {
                    found_match = Some(past_frame_idx as i32);
                    break; 
                }
            }

            if let Some(start_f) = found_match {
                let _ = status_tx.send(LoopStatus::Found(start_f, current_frame));
                break;
            }

            // Save state and continue
            frame_states.push(current_state);
            current_frame += 1;

            // Report progress
            if current_frame % 5 == 0 {
                let _ = status_tx.send(LoopStatus::Searching(current_frame as usize));
            }
            
            // Yield to prevent freezing
            if current_frame % 100 == 0 {
                thread::sleep(Duration::from_millis(1));
            }
        }
    });
}