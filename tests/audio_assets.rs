//! AUDIO-01: the Floor 01 pack is complete, intact and fully mapped.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use dungeon_floorbreak::audio::{AmbienceState, CueBank, CueId, MusicState, PACK_DIR, StyleBank};
use serde_json::Value;

fn pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(PACK_DIR)
}

fn bank() -> CueBank {
    CueBank::load(&pack()).expect("the cue bank loads from the committed pack")
}

#[test]
fn every_cue_state_and_style_resolves_to_existing_ogg_files() {
    let bank = bank();
    let pack = pack();
    let exists = |rel: &str| {
        assert!(rel.ends_with(".ogg"), "{rel} is not an Ogg file");
        assert!(pack.join(rel).is_file(), "{rel} is missing");
    };
    for style in [StyleBank::Arcade, StyleBank::Cinematic, StyleBank::Hybrid] {
        for cue in CueId::ALL {
            let takes = bank.cue_assets(cue, style);
            assert!(
                !takes.is_empty(),
                "{} has no takes for {style:?}",
                cue.name()
            );
            takes.iter().for_each(|t| exists(t));
        }
    }
    for state in MusicState::ALL {
        let plan = bank.music_plan(state).expect("music plan");
        exists(&plan.looped);
        plan.one_shots.iter().for_each(|t| exists(t));
    }
    for state in AmbienceState::ALL {
        exists(bank.ambience_asset(state).expect("ambience bed"));
    }
    assert!(!bank.referenced_assets().is_empty());
}

#[test]
fn style_banks_change_the_hit_confirm_takes_only() {
    let bank = bank();
    let hit = |s| bank.cue_assets(CueId::PlayerKickHitConfirmed, s).to_vec();
    assert_ne!(hit(StyleBank::Arcade), hit(StyleBank::Hybrid));
    assert_ne!(hit(StyleBank::Cinematic), hit(StyleBank::Hybrid));
    assert_eq!(hit(StyleBank::Hybrid).len(), 3, "three takes per bank");
    let swing = |s| bank.cue_assets(CueId::PlayerKickSwing, s).to_vec();
    assert_eq!(swing(StyleBank::Arcade), swing(StyleBank::Cinematic));
}

#[test]
fn the_mixer_follows_the_pack_and_the_brief() {
    let m = bank().mixer;
    assert_eq!(m.default_style, StyleBank::Hybrid);
    assert_eq!(m.max_major_contacts, 2);
    assert!((100.0..=180.0).contains(&(m.duck_seconds * 1000.0)));
    assert!(
        (-4.0..=-2.5).contains(&m.duck_db),
        "about 3 dB, got {}",
        m.duck_db
    );
    assert!(m.duck_gain() < 1.0);
}

#[test]
fn cue_events_sit_on_the_sfx_bus_in_the_pack() {
    let bank = bank();
    for cue in CueId::ALL {
        assert_eq!(bank.cue_bus(cue), Some("sfx"), "{}", cue.name());
    }
}

#[test]
fn engine_cue_names_stay_ip_neutral() {
    for cue in CueId::ALL {
        let name = cue.name().to_ascii_lowercase();
        for fan in ["carl", "goblin", "donut", "mongo", "crawl", "dozer"] {
            assert!(!name.contains(fan), "{name} contains fan-layer name {fan}");
        }
    }
}

// ------------------------------------------------------------ manifest --

#[test]
fn every_runtime_asset_in_the_manifest_exists_and_matches_its_sha256() {
    let pack = pack();
    let text = std::fs::read_to_string(pack.join("docs/asset_manifest.json")).unwrap();
    let manifest: Value = serde_json::from_str(&text).unwrap();
    let assets = manifest["assets"].as_array().expect("assets array");
    assert_eq!(assets.len(), 38, "the pack lists 38 runtime files");
    let mut listed = HashSet::new();
    for asset in assets {
        let rel = asset["path"].as_str().unwrap();
        let want = asset["sha256"].as_str().unwrap();
        let bytes = std::fs::read(pack.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(sha256_hex(&bytes), want, "sha256 of {rel}");
        listed.insert(rel.to_string());
    }
    // Every Ogg under music/, sfx/ and variants/ is in the manifest.
    let mut on_disk = Vec::new();
    for dir in ["music", "sfx", "variants"] {
        collect_ogg(&pack.join(dir), &pack, &mut on_disk);
    }
    for rel in on_disk {
        assert!(listed.contains(&rel), "{rel} is not in the manifest");
    }
}

fn collect_ogg(dir: &Path, root: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_ogg(&path, root, out);
        } else if path.extension().is_some_and(|e| e == "ogg") {
            let rel = path.strip_prefix(root).unwrap();
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// Minimal SHA-256 (FIPS 180-4), so the test needs no new crate.
fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(v);
        }
    }
    h.iter().map(|v| format!("{v:08x}")).collect()
}

#[test]
fn the_sha256_helper_matches_known_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
