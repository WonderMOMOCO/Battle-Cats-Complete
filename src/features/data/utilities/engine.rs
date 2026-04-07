use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicI32, AtomicBool, AtomicUsize, Ordering};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use rayon::prelude::*;

use crate::features::data::utilities::{apk, crypto, audit, manifest, router, rules, chrono};
use crate::global::io::patterns;
use crate::features::settings::logic::exceptions::RuleHandling;

#[derive(Clone)]
struct UniversalTask {
    pack_path: PathBuf,
    original_name: String,
    final_name: String,
    byte_offset: u64,
    byte_size: usize,
    region_code: String,
    chrono_score: u64,
    is_loose: bool,
}

struct DecryptedCandidate {
    task: UniversalTask,
    clean_data: Vec<u8>,
    true_weight: usize,
}

fn determine_region_code(filename: &str, folder_region: &str) -> String {
    if folder_region != "en" { return folder_region.to_string(); }
    for &(language_code, _) in patterns::APP_LANGUAGES {
        if language_code == "en" { continue; } 
        let language_suffix = format!("_{}", language_code);
        if filename.contains(&language_suffix) { return language_code.to_string(); }
    }
    "en".to_string()
}

fn get_region_priority(region_code: &str) -> u8 {
    match region_code { "ja" => 4, "en" => 3, "tw" => 2, "ko" => 1, _ => 0 }
}

fn cleanup_temporary_directories(directories: &[PathBuf]) {
    for directory in directories { let _ = fs::remove_dir_all(directory); }
}

pub fn run_universal_import(
    source_directories: &[PathBuf], 
    status_sender: &Sender<String>, 
    abort_flag: &Arc<AtomicBool>, 
    progress_current: &Arc<AtomicUsize>, 
    progress_maximum: &Arc<AtomicUsize>
) -> Result<(), String> {
    
    let game_root_path = Path::new("game");
    let meta_directory_path = game_root_path.join("meta");
    let pack_manifest_path = meta_directory_path.join("pack.json");
    let file_manifest_path = meta_directory_path.join("file.json");
    
    let mut global_pack_registry: HashMap<String, HashMap<String, manifest::PackRecord>> = manifest::load(&pack_manifest_path);
    let mut global_file_ledger: HashMap<String, manifest::ManifestEntry> = manifest::load(&file_manifest_path);
    
    let asset_router_utility = router::AssetRouter::new(game_root_path);
    let (compiled_regex_set, compiled_exception_rules) = rules::compile();

    let _ = status_sender.send("Collecting game data...".to_string());
    
    let mut universal_task_map: HashMap<String, Vec<UniversalTask>> = HashMap::new();
    let mut global_temporary_directories: Vec<PathBuf> = Vec::new();
    let mut current_pack_hashes: HashMap<String, HashMap<String, manifest::PackRecord>> = HashMap::new();
    
    let mut has_notified_extraction = false;

    for source_directory in source_directories {
        if abort_flag.load(Ordering::Relaxed) { 
            cleanup_temporary_directories(&global_temporary_directories);
            return Err("Job Aborted".into()); 
        }
        
        let mut folder_region_name = source_directory.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        if folder_region_name == "files" {
            if let Some(parent_directory) = source_directory.parent() {
                folder_region_name = parent_directory.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
            }
        }
        
        let current_region_code = match folder_region_name.as_str() {
            s if s.ends_with("tw") => "tw",
            s if s.ends_with("kr") || s.ends_with("ko") => "ko",
            s if s.ends_with("en") => "en",
            s if s.ends_with("battlecats") => "ja",
            _ => "en",
        };

        let mut discovered_list_files = Vec::new();
        let mut discovered_apk_files = Vec::new();
        let mut discovered_audio_files = Vec::new();
        
        let _ = apk::find_files(source_directory, &mut discovered_list_files, &mut discovered_apk_files, &mut discovered_audio_files);
        
        if !discovered_apk_files.is_empty() && !has_notified_extraction {
            let _ = status_sender.send("Extracting update data...".to_string());
            has_notified_extraction = true;
        }
        
        let (mut new_list_paths, mut new_temp_dirs) = apk::extract_all(&discovered_apk_files);
        
        discovered_list_files.append(&mut new_list_paths);
        global_temporary_directories.append(&mut new_temp_dirs);
        
        let calculated_chrono_score = chrono::calculate(source_directory, &global_temporary_directories);

        for audio_path in discovered_audio_files {
            let filename = audio_path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let byte_size = fs::metadata(&audio_path).map(|m| m.len() as usize).unwrap_or(0);
            
            let matched_user_rule = compiled_regex_set.matches(&filename).into_iter().next().map(|index| &compiled_exception_rules[index]);
            if let Some(rule) = matched_user_rule { if rule.handling == RuleHandling::Ignore { continue; } }
            
            let mut final_resolved_filename = filename.clone();
            
            if let Some(rule) = matched_user_rule {
                if rule.languages.values().any(|&is_active| is_active) {
                    let asset_path_object = Path::new(&filename);
                    let asset_stem_string = asset_path_object.file_stem().unwrap().to_string_lossy();
                    let asset_extension_string = asset_path_object.extension().unwrap_or_default().to_string_lossy();
                    
                    let mut cleaned_stem = asset_stem_string.to_string();
                    for &(code, _) in patterns::APP_LANGUAGES {
                        let suffix = format!("_{}", code);
                        if cleaned_stem.ends_with(&suffix) { cleaned_stem = cleaned_stem.trim_end_matches(&suffix).to_string(); break; }
                    }
                    
                    let is_region_enabled = rule.languages.get(current_region_code).copied().unwrap_or(false);
                    if rule.handling == RuleHandling::Only && !is_region_enabled { continue; }
                    let is_single = rule.handling == RuleHandling::Only && rule.languages.values().filter(|&&is_active| is_active).count() == 1;
                    
                    if is_region_enabled {
                        if is_single {
                            final_resolved_filename = if asset_extension_string.is_empty() { cleaned_stem } else { format!("{}.{}", cleaned_stem, asset_extension_string) };
                        } else if !current_region_code.is_empty() {
                            final_resolved_filename = if asset_extension_string.is_empty() { format!("{}_{}", cleaned_stem, current_region_code) } else { format!("{}_{}.{}", cleaned_stem, current_region_code, asset_extension_string) };
                        }
                    }
                }
            }

            let extraction_task = UniversalTask {
                pack_path: audio_path,
                original_name: filename,
                final_name: final_resolved_filename.clone(),
                byte_offset: 0,
                byte_size,
                region_code: current_region_code.to_string(),
                chrono_score: calculated_chrono_score,
                is_loose: true,
            };
            universal_task_map.entry(final_resolved_filename).or_insert_with(Vec::new).push(extraction_task);
        }

        for item_path in discovered_list_files {
            let corresponding_pack_path = item_path.with_extension("pack");
            if !corresponding_pack_path.exists() { continue; }
            
            let pack_filename = corresponding_pack_path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let final_region_code = determine_region_code(&pack_filename, current_region_code);
            
            let region_pack_map = current_pack_hashes.entry(final_region_code.clone()).or_insert_with(HashMap::new);

            if !region_pack_map.contains_key(&pack_filename) {
                if let Ok(pack_hash_value) = manifest::hash_file(&corresponding_pack_path) {
                    region_pack_map.insert(pack_filename.clone(), manifest::PackRecord { checksum: pack_hash_value });
                }
            }
            
            let Ok(list_file_data) = fs::read(&item_path) else { continue; };
            
            let decrypted_list_content = {
                let pack_key = crypto::get_md5_key("pack");
                if let Ok(bytes) = crypto::decrypt_ecb_with_key(&list_file_data, &pack_key) { String::from_utf8(bytes).ok() } 
                else {
                    let battlecats_key = crypto::get_md5_key("battlecats");
                    if let Ok(bytes) = crypto::decrypt_ecb_with_key(&list_file_data, &battlecats_key) { String::from_utf8(bytes).ok() } 
                    else { None }
                }
            };

            let Some(decoded_string_content) = decrypted_list_content else { continue; };
            
            for text_line in decoded_string_content.lines() {
                let parts: Vec<&str> = text_line.split(',').collect();
                if parts.len() < 3 { continue; }
                
                let raw_asset_name = parts[0];
                let byte_offset_value: u64 = parts[1].parse().unwrap_or(0);
                let byte_size_value: usize = parts[2].parse().unwrap_or(0);

                let matched_user_rule = compiled_regex_set.matches(raw_asset_name).into_iter().next().map(|index| &compiled_exception_rules[index]);
                if let Some(rule) = matched_user_rule { if rule.handling == RuleHandling::Ignore { continue; } }

                let mut final_resolved_filename = raw_asset_name.to_string();
                
                if let Some(rule) = matched_user_rule {
                    if rule.languages.values().any(|&is_active| is_active) {
                        let asset_path_object = Path::new(raw_asset_name);
                        let asset_stem_string = asset_path_object.file_stem().unwrap().to_string_lossy();
                        let asset_extension_string = asset_path_object.extension().unwrap_or_default().to_string_lossy();
                        
                        let mut cleaned_stem = asset_stem_string.to_string();
                        for &(code, _) in patterns::APP_LANGUAGES {
                            let suffix = format!("_{}", code);
                            if cleaned_stem.ends_with(&suffix) { cleaned_stem = cleaned_stem.trim_end_matches(&suffix).to_string(); break; }
                        }
                        
                        let is_region_enabled = rule.languages.get(final_region_code.as_str()).copied().unwrap_or(false);
                        if rule.handling == RuleHandling::Only && !is_region_enabled { continue; }
                        let is_single = rule.handling == RuleHandling::Only && rule.languages.values().filter(|&&is_active| is_active).count() == 1;
                        
                        if is_region_enabled {
                            if is_single {
                                final_resolved_filename = if asset_extension_string.is_empty() { cleaned_stem } else { format!("{}.{}", cleaned_stem, asset_extension_string) };
                            } else if !final_region_code.is_empty() {
                                final_resolved_filename = if asset_extension_string.is_empty() { format!("{}_{}", cleaned_stem, final_region_code) } else { format!("{}_{}.{}", cleaned_stem, final_region_code, asset_extension_string) };
                            }
                        }
                    }
                }

                let extraction_task = UniversalTask {
                    pack_path: corresponding_pack_path.clone(),
                    original_name: raw_asset_name.to_string(),
                    final_name: final_resolved_filename.clone(),
                    byte_offset: byte_offset_value,
                    byte_size: byte_size_value,
                    region_code: final_region_code.clone(),
                    chrono_score: calculated_chrono_score,
                    is_loose: false,
                };

                universal_task_map.entry(final_resolved_filename).or_insert_with(Vec::new).push(extraction_task);
            }
        }
    }

    let mut final_extraction_queue: Vec<(String, Vec<UniversalTask>, PathBuf)> = Vec::new();
    
    for (resolved_filename, duplicate_tasks) in universal_task_map {
        let mut tasks_by_region: HashMap<String, Vec<UniversalTask>> = HashMap::new();
        for processing_task in duplicate_tasks {
            tasks_by_region.entry(processing_task.region_code.clone()).or_insert_with(Vec::new).push(processing_task);
        }

        let mut regional_winners_to_decrypt: Vec<UniversalTask> = Vec::new();
        for (_, mut regional_tasks) in tasks_by_region {
            regional_tasks.sort_by_key(|task| task.chrono_score);
            if let Some(newest_task_for_region) = regional_tasks.pop() {
                regional_winners_to_decrypt.push(newest_task_for_region);
            }
        }

        let representative_candidate = regional_winners_to_decrypt.first().unwrap();
        let target_destination_path = asset_router_utility.resolve_destination(&representative_candidate.original_name, &resolved_filename);

        let mut requires_memory_decryption = false;
        
        if let Some(existing_manifest_entry) = global_file_ledger.get(&resolved_filename) {
            for candidate in &regional_winners_to_decrypt {
                if candidate.is_loose {
                    if candidate.byte_size > existing_manifest_entry.encrypted {
                        requires_memory_decryption = true;
                        break;
                    }
                } else {
                    let pack_filename = candidate.pack_path.file_name().unwrap_or_default().to_string_lossy().into_owned();
                    
                    let newly_calculated_hash = current_pack_hashes
                        .get(&candidate.region_code)
                        .and_then(|region_map| region_map.get(&pack_filename))
                        .map(|record| record.checksum);
                        
                    let saved_manifest_hash = global_pack_registry
                        .get(&candidate.region_code)
                        .and_then(|region_map| region_map.get(&pack_filename))
                        .map(|record| record.checksum);

                    if newly_calculated_hash.is_none() || newly_calculated_hash != saved_manifest_hash {
                        requires_memory_decryption = true;
                        break;
                    }
                }
            }

            if !requires_memory_decryption && !target_destination_path.exists() {
                requires_memory_decryption = true;
            }
        } else {
            requires_memory_decryption = true;
        }
        
        if requires_memory_decryption {
            final_extraction_queue.push((resolved_filename, regional_winners_to_decrypt, target_destination_path)); 
        }
    }

    if final_extraction_queue.is_empty() {
        let _ = status_sender.send("Workspace is completely up to date.".to_string());
        progress_maximum.store(0, Ordering::Relaxed);
        
        for (region_key, pack_map) in current_pack_hashes {
            let region_entry = global_pack_registry.entry(region_key).or_insert_with(HashMap::new);
            region_entry.extend(pack_map);
        }
        
        manifest::save(&pack_manifest_path, &global_pack_registry);
        cleanup_temporary_directories(&global_temporary_directories);
        return Ok(());
    }

    progress_maximum.store(final_extraction_queue.len(), Ordering::Relaxed);
    progress_current.store(0, Ordering::Relaxed);
    
    let successfully_extracted_count = AtomicI32::new(0);
    let console_update_interval = (final_extraction_queue.len() / 100).max(10);

    let _ = status_sender.send(format!("Comparing and organizing {} game files...", final_extraction_queue.len()));

    let updated_manifest_entries: Vec<(String, manifest::ManifestEntry)> = final_extraction_queue.into_par_iter().filter_map(|(resolved_filename, regional_tasks_to_decrypt, target_destination_path)| {
        if abort_flag.load(Ordering::Relaxed) { return None; }

        let mut decrypted_candidates: Vec<DecryptedCandidate> = Vec::new();

        for processing_task in regional_tasks_to_decrypt {
            if processing_task.is_loose {
                if let Ok(raw_data) = fs::read(&processing_task.pack_path) {
                    decrypted_candidates.push(DecryptedCandidate {
                        task: processing_task.clone(),
                        clean_data: raw_data.clone(),
                        true_weight: raw_data.len(), 
                    });
                }
                continue;
            }

            let Ok(mut input_pack_file) = fs::File::open(&processing_task.pack_path) else { continue; };
            let memory_aligned_size = if processing_task.byte_size % 16 == 0 { processing_task.byte_size } else { ((processing_task.byte_size / 16) + 1) * 16 };
            let mut encrypted_byte_buffer = vec![0u8; memory_aligned_size];
            if input_pack_file.seek(SeekFrom::Start(processing_task.byte_offset)).is_err() { continue; }
            if input_pack_file.read_exact(&mut encrypted_byte_buffer).is_err() { continue; }

            if let Ok((decrypted_byte_vector, _)) = crypto::decrypt_pack_chunk(&encrypted_byte_buffer, &processing_task.original_name) {
                let strict_size_limit = std::cmp::min(processing_task.byte_size, decrypted_byte_vector.len());
                let exact_data_slice = &decrypted_byte_vector[..strict_size_limit];

                let calculated_true_weight = audit::calculate_true_weight(exact_data_slice, &processing_task.final_name);
                let cleaned_data_vector = audit::strip_carriage_returns(exact_data_slice, &processing_task.final_name);

                decrypted_candidates.push(DecryptedCandidate {
                    task: processing_task,
                    clean_data: cleaned_data_vector,
                    true_weight: calculated_true_weight,
                });
            }
        }

        if decrypted_candidates.is_empty() {
            progress_current.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        decrypted_candidates.sort_by(|candidate_a, candidate_b| {
            let weight_cmp = candidate_a.true_weight.cmp(&candidate_b.true_weight);
            if weight_cmp == std::cmp::Ordering::Equal {
                let chrono_cmp = candidate_a.task.chrono_score.cmp(&candidate_b.task.chrono_score);
                if chrono_cmp == std::cmp::Ordering::Equal { get_region_priority(&candidate_a.task.region_code).cmp(&get_region_priority(&candidate_b.task.region_code)) } 
                else { chrono_cmp }
            } else { weight_cmp }
        });

        let winning_candidate = decrypted_candidates.pop().unwrap();
        let winning_checksum = manifest::hash(&winning_candidate.clean_data);
        let mut should_write_to_disk = true;

        if let Some(existing_manifest_entry) = global_file_ledger.get(&resolved_filename) {
            if winning_candidate.true_weight < existing_manifest_entry.weight {
                should_write_to_disk = false; 
            } else if winning_candidate.true_weight == existing_manifest_entry.weight && winning_checksum == existing_manifest_entry.checksum {
                if target_destination_path.exists() { should_write_to_disk = false; }
            }
        }

        if should_write_to_disk {
            if let Some(parent_directory) = target_destination_path.parent() { let _ = fs::create_dir_all(parent_directory); }
            let _ = fs::write(&target_destination_path, &winning_candidate.clean_data);
            
            let current_extracted_total = successfully_extracted_count.fetch_add(1, Ordering::Relaxed) + 1;
            if current_extracted_total as usize % console_update_interval == 0 { let _ = status_sender.send(format!("Processed {} files | Routing: {}", current_extracted_total, resolved_filename)); }

            progress_current.fetch_add(1, Ordering::Relaxed);

            return Some((resolved_filename.clone(), manifest::ManifestEntry {
                winner: winning_candidate.task.region_code,
                weight: winning_candidate.true_weight,
                size: winning_candidate.clean_data.len(),
                encrypted: winning_candidate.task.byte_size,
                checksum: winning_checksum,
            }));
        } 

        progress_current.fetch_add(1, Ordering::Relaxed);
        None
        
    }).collect();

    if abort_flag.load(Ordering::Relaxed) { 
        cleanup_temporary_directories(&global_temporary_directories);
        return Err("Job Aborted".into()); 
    }
    
    for (filename_key, entry_data) in updated_manifest_entries { 
        global_file_ledger.insert(filename_key, entry_data); 
    }
    
    for (region_key, pack_map) in current_pack_hashes {
        let region_entry = global_pack_registry.entry(region_key).or_insert_with(HashMap::new);
        region_entry.extend(pack_map);
    }
    
    manifest::save(&pack_manifest_path, &global_pack_registry);
    manifest::save(&file_manifest_path, &global_file_ledger);
    
    cleanup_temporary_directories(&global_temporary_directories);

    let _ = status_sender.send("Files successfully organized and updated.".to_string());
    Ok(())
}