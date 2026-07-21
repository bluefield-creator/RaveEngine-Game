use bevy::prelude::*;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

pub const CURRENT_VRTX_VERSION: u32 = 7;

fn read_f32(reader: &mut impl Read) -> std::io::Result<f32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(f32::from_le_bytes(bytes))
}

fn read_file_with_limit(path: &str, max_size: u64) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut data = Vec::new();
    let read_limit = max_size.checked_add(1).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "File size limit overflow")
    })?;
    (&mut file).take(read_limit).read_to_end(&mut data)?;
    if data.len() as u64 > max_size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "File exceeds maximum allowed size",
        ));
    }
    Ok(data)
}

#[derive(Debug, Clone)]
pub struct VrtxBrick {
    pub node_id: u64,
    pub parent_node_id: Option<u64>,
    pub name: String,
    pub transform: Transform,
    pub shape: crate::common::game::bricks::components::BrickShape,
    pub color: Color,
    pub physics_enabled: bool,
    pub bounciness: f32,
    pub player_can_collide: bool,
    pub friction: f32,
    pub gravity_scale: f32,
    pub mass: f32,
}

#[derive(Debug, Clone)]
pub struct VrtxScript {
    pub node_id: u64,
    pub parent_node_id: Option<u64>,
    pub name: String,
    pub script_type: u8,
    pub code: String,
    pub parent_name: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct VrtxSettings {
    pub ssao: bool,
    pub ssao_quality: crate::common::core::performance::AmbientOcclusionQuality,
    pub contact_shadows: bool,
    pub contact_shadow_length: f32,
    pub bloom: bool,
    pub bloom_intensity: f32,
    pub shadow_quality: crate::common::core::performance::ShadowQuality,
    pub soft_shadows: bool,
    pub anti_aliasing: crate::common::core::performance::AntiAliasing,
    pub exposure_ev100: f32,
}

impl Default for VrtxSettings {
    fn default() -> Self {
        Self::from_graphics(&crate::common::core::performance::GraphicsSettings::default())
    }
}

impl VrtxSettings {
    pub fn from_graphics(settings: &crate::common::core::performance::GraphicsSettings) -> Self {
        Self {
            ssao: settings.ssao,
            ssao_quality: settings.ssao_quality,
            contact_shadows: settings.contact_shadows,
            contact_shadow_length: settings.contact_shadow_length,
            bloom: settings.bloom,
            bloom_intensity: settings.bloom_intensity,
            shadow_quality: settings.shadow_quality,
            soft_shadows: settings.soft_shadows,
            anti_aliasing: settings.anti_aliasing,
            exposure_ev100: settings.exposure_ev100,
        }
    }

    pub fn apply_to(&self, settings: &mut crate::common::core::performance::GraphicsSettings) {
        settings.ssao = self.ssao;
        settings.ssao_quality = self.ssao_quality;
        settings.contact_shadows = self.contact_shadows;
        settings.contact_shadow_length = self.contact_shadow_length;
        settings.bloom = self.bloom;
        settings.bloom_intensity = self.bloom_intensity;
        settings.shadow_quality = self.shadow_quality;
        settings.soft_shadows = self.soft_shadows;
        settings.anti_aliasing = if self.ssao
            && matches!(
                self.anti_aliasing,
                crate::common::core::performance::AntiAliasing::Msaa2
                    | crate::common::core::performance::AntiAliasing::Msaa4
                    | crate::common::core::performance::AntiAliasing::Msaa8
            ) {
            crate::common::core::performance::AntiAliasing::Fxaa
        } else {
            self.anti_aliasing
        };
        settings.exposure_ev100 = self.exposure_ev100;
    }
}

#[derive(Debug, Clone)]
pub struct VrtxFileState {
    pub version: u32,
    pub gravity: Vec3,
    pub settings: VrtxSettings,
    pub lighting: crate::common::game::environment::lighting::LightingService,
    pub camera_transform: Transform,
    pub bricks: Vec<VrtxBrick>,
    pub scripts: Vec<VrtxScript>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum GodotVariant {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vector2(Vec2),
    Vector3(Vec3),
    Color(Color),
    Dictionary(std::collections::HashMap<String, GodotVariant>),
    Array(Vec<GodotVariant>),
}

struct GodotParser<'a> {
    data: &'a [u8],
    offset: usize,
    variants_parsed: usize,
}

const MAX_GODOT_COLLECTION_COUNT: u32 = 100_000;
const MAX_GODOT_RECURSION_DEPTH: usize = 64;
const MAX_GODOT_STRING_LENGTH: usize = 1_048_576;

impl<'a> GodotParser<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            offset: 0,
            variants_parsed: 0,
        }
    }

    fn read_u32(&mut self) -> std::io::Result<u32> {
        if self
            .offset
            .checked_add(4)
            .is_none_or(|end| end > self.data.len())
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Unexpected EOF",
            ));
        }
        let val = u32::from_le_bytes([
            self.data[self.offset],
            self.data[self.offset + 1],
            self.data[self.offset + 2],
            self.data[self.offset + 3],
        ]);
        self.offset += 4;
        Ok(val)
    }

    fn read_f32(&mut self) -> std::io::Result<f32> {
        if self
            .offset
            .checked_add(4)
            .is_none_or(|end| end > self.data.len())
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Unexpected EOF",
            ));
        }
        let val = f32::from_le_bytes([
            self.data[self.offset],
            self.data[self.offset + 1],
            self.data[self.offset + 2],
            self.data[self.offset + 3],
        ]);
        self.offset += 4;
        Ok(val)
    }

    fn read_f64(&mut self) -> std::io::Result<f64> {
        if self
            .offset
            .checked_add(8)
            .is_none_or(|end| end > self.data.len())
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Unexpected EOF",
            ));
        }
        let val = f64::from_le_bytes([
            self.data[self.offset],
            self.data[self.offset + 1],
            self.data[self.offset + 2],
            self.data[self.offset + 3],
            self.data[self.offset + 4],
            self.data[self.offset + 5],
            self.data[self.offset + 6],
            self.data[self.offset + 7],
        ]);
        self.offset += 8;
        Ok(val)
    }

    fn read_bytes(&mut self, len: usize) -> std::io::Result<&'a [u8]> {
        if self
            .offset
            .checked_add(len)
            .is_none_or(|end| end > self.data.len())
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Unexpected EOF",
            ));
        }
        let slice = &self.data[self.offset..self.offset + len];
        self.offset += len;
        Ok(slice)
    }

    fn parse_variant(&mut self) -> std::io::Result<GodotVariant> {
        self.parse_variant_at_depth(0)
    }

    fn parse_variant_at_depth(&mut self, depth: usize) -> std::io::Result<GodotVariant> {
        if depth > MAX_GODOT_RECURSION_DEPTH {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Godot variant recursion depth exceeds maximum allowed",
            ));
        }
        self.variants_parsed = self.variants_parsed.checked_add(1).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Godot variant count overflow",
            )
        })?;
        if self.variants_parsed > MAX_GODOT_COLLECTION_COUNT as usize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Godot variant count exceeds maximum allowed",
            ));
        }
        let start_offset = self.offset;
        let type_header = self.read_u32()?;
        let type_id = type_header & 0xFFFF;
        let flags = (type_header >> 16) & 0xFF;
        let is_64 = flags == 1;

        trace!(
            "parse_variant at offset {}: type_id={}, flags={}, is_64={}",
            start_offset, type_id, flags, is_64
        );

        let var = match type_id {
            0 | 25 | 26 => Ok(GodotVariant::Nil),
            1 => {
                let val = self.read_u32()?;
                Ok(GodotVariant::Bool(val != 0))
            }
            2 => {
                let bytes_to_read = if is_64 { 8 } else { 4 };
                let bytes = self.read_bytes(bytes_to_read)?;
                if is_64 {
                    let mut b = [0u8; 8];
                    b.copy_from_slice(bytes);
                    Ok(GodotVariant::Int(i64::from_le_bytes(b)))
                } else {
                    let mut b = [0u8; 4];
                    b.copy_from_slice(bytes);
                    Ok(GodotVariant::Int(i32::from_le_bytes(b) as i64))
                }
            }
            3 => {
                if is_64 {
                    let val = self.read_f64()?;
                    Ok(GodotVariant::Float(val))
                } else {
                    let val = self.read_f32()?;
                    Ok(GodotVariant::Float(val as f64))
                }
            }
            4 | 21 | 22 => {
                let len = self.read_u32()? as usize;
                if len > MAX_GODOT_STRING_LENGTH {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Godot string exceeds maximum allowed length",
                    ));
                }
                let padded_len = len.checked_add(3).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Godot string length overflow",
                    )
                })? & !3;
                let str_bytes = self.read_bytes(len)?;
                let _padding = self.read_bytes(padded_len - len)?;
                let string = String::from_utf8_lossy(str_bytes).into_owned();
                Ok(GodotVariant::String(string))
            }
            5 => {
                let x = if is_64 {
                    self.read_f64()? as f32
                } else {
                    self.read_f32()?
                };
                let y = if is_64 {
                    self.read_f64()? as f32
                } else {
                    self.read_f32()?
                };
                Ok(GodotVariant::Vector2(Vec2::new(x, y)))
            }
            6 => {
                let len = if is_64 { 32 } else { 16 };
                let _bytes = self.read_bytes(len)?;
                Ok(GodotVariant::Nil)
            }
            7 => {
                let _bytes = self.read_bytes(8)?;
                Ok(GodotVariant::Nil)
            }
            8 => {
                let _bytes = self.read_bytes(16)?;
                Ok(GodotVariant::Nil)
            }
            9 => {
                let x = if is_64 {
                    self.read_f64()? as f32
                } else {
                    self.read_f32()?
                };
                let y = if is_64 {
                    self.read_f64()? as f32
                } else {
                    self.read_f32()?
                };
                let z = if is_64 {
                    self.read_f64()? as f32
                } else {
                    self.read_f32()?
                };
                Ok(GodotVariant::Vector3(Vec3::new(x, y, z)))
            }
            10 => {
                let _bytes = self.read_bytes(12)?;
                Ok(GodotVariant::Nil)
            }
            11 => {
                let _bytes = if is_64 {
                    self.read_bytes(32)?
                } else {
                    self.read_bytes(16)?
                };
                Ok(GodotVariant::Nil)
            }
            12 => {
                let _bytes = if is_64 {
                    self.read_bytes(32)?
                } else {
                    self.read_bytes(16)?
                };
                Ok(GodotVariant::Nil)
            }
            13 => {
                let _bytes = if is_64 {
                    self.read_bytes(32)?
                } else {
                    self.read_bytes(16)?
                };
                Ok(GodotVariant::Nil)
            }
            14 => {
                let _bytes = if is_64 {
                    self.read_bytes(48)?
                } else {
                    self.read_bytes(24)?
                };
                Ok(GodotVariant::Nil)
            }
            15 => {
                let _bytes = if is_64 {
                    self.read_bytes(72)?
                } else {
                    self.read_bytes(36)?
                };
                Ok(GodotVariant::Nil)
            }
            16 => {
                let _bytes = if is_64 {
                    self.read_bytes(96)?
                } else {
                    self.read_bytes(48)?
                };
                Ok(GodotVariant::Nil)
            }
            17 => {
                let _bytes = if is_64 {
                    self.read_bytes(128)?
                } else {
                    self.read_bytes(64)?
                };
                Ok(GodotVariant::Nil)
            }
            18 => {
                let len = if is_64 { 96 } else { 48 };
                let _bytes = self.read_bytes(len)?;
                Ok(GodotVariant::Nil)
            }
            19 => {
                let len = if is_64 { 128 } else { 64 };
                let _bytes = self.read_bytes(len)?;
                Ok(GodotVariant::Nil)
            }
            20 => {
                let r = self.read_f32()?;
                let g = self.read_f32()?;
                let b = self.read_f32()?;
                let a = self.read_f32()?;
                Ok(GodotVariant::Color(Color::Srgba(Srgba::new(r, g, b, a))))
            }
            23 => {
                let _bytes = self.read_bytes(8)?;
                Ok(GodotVariant::Nil)
            }
            24 => {
                let object_type = self.read_u32()?;
                if object_type == 1 {
                    let len = self.read_u32()? as usize;
                    if len > MAX_GODOT_STRING_LENGTH {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Godot class name exceeds maximum allowed length",
                        ));
                    }
                    let padded_len = len.checked_add(3).ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Godot class name length overflow",
                        )
                    })? & !3;
                    let _class_name = self.read_bytes(len)?;
                    let _padding = self.read_bytes(padded_len - len)?;
                    let prop_count = self.read_u32()?;
                    if prop_count > MAX_GODOT_COLLECTION_COUNT {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Godot property count exceeds maximum allowed",
                        ));
                    }
                    for _ in 0..prop_count {
                        let _name = self.parse_variant_at_depth(depth + 1)?;
                        let _val = self.parse_variant_at_depth(depth + 1)?;
                    }
                } else if object_type == 2 {
                    let _bytes = self.read_bytes(8)?;
                }
                Ok(GodotVariant::Nil)
            }
            27 => {
                let count_header = self.read_u32()?;
                let count = count_header & 0x7FFFFFFF;
                if count > MAX_GODOT_COLLECTION_COUNT {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Godot dictionary count exceeds maximum allowed",
                    ));
                }
                trace!(
                    "parse_variant at offset {}: parsing dictionary with {} elements",
                    start_offset, count
                );
                let mut dict = std::collections::HashMap::new();
                for i in 0..count {
                    let key_var = self.parse_variant_at_depth(depth + 1)?;
                    let val_var = self.parse_variant_at_depth(depth + 1)?;
                    trace!(
                        "parse_variant dictionary element {}: key={:?}, val_type={:?}",
                        i, key_var, val_var
                    );
                    if let GodotVariant::String(key_str) = key_var {
                        dict.insert(key_str, val_var);
                    }
                }
                Ok(GodotVariant::Dictionary(dict))
            }
            28 => {
                let count_header = self.read_u32()?;
                let count = count_header & 0x7FFFFFFF;
                if count > MAX_GODOT_COLLECTION_COUNT {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Godot array count exceeds maximum allowed",
                    ));
                }
                trace!(
                    "parse_variant at offset {}: parsing array with {} elements",
                    start_offset, count
                );
                let mut arr = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let val_var = self.parse_variant_at_depth(depth + 1)?;
                    arr.push(val_var);
                }
                Ok(GodotVariant::Array(arr))
            }
            _ => {
                error!(
                    "parse_variant at offset {}: Unsupported Godot variant type: {}",
                    start_offset, type_id
                );
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unsupported Godot variant type: {}", type_id),
                ))
            }
        };

        if let Ok(ref _value) = var {
            trace!(
                "parse_variant at offset {} successfully parsed",
                start_offset
            );
        }
        var
    }
}

fn decompress_gcpf_file(data: &[u8]) -> std::io::Result<Vec<u8>> {
    if data.len() < 16 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "GCPF: File too short",
        ));
    }
    if &data[0..4] != b"GCPF" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "GCPF: Invalid magic",
        ));
    }

    let comp_mode = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let block_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
    let uncompressed_size = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;

    if uncompressed_size as u64 > VrtxFileState::MAX_FILE_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "GCPF: Declared decompressed size exceeds maximum allowed",
        ));
    }

    debug!(
        "GCPF decompress: mode={}, block_size={}, uncompressed_size={}",
        comp_mode, block_size, uncompressed_size
    );

    if block_size == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "GCPF: Block size is zero",
        ));
    }

    let num_blocks = uncompressed_size.div_ceil(block_size);
    if num_blocks > MAX_GODOT_COLLECTION_COUNT as usize {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "GCPF: Block count exceeds maximum allowed",
        ));
    }
    let header_size = num_blocks
        .checked_mul(4)
        .and_then(|size| 16usize.checked_add(size))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "GCPF: Header size overflow",
            )
        })?;
    debug!(
        "GCPF decompress: num_blocks={}, header_size={}",
        num_blocks, header_size
    );

    if data.len() < header_size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "GCPF: Header size exceeds file length",
        ));
    }

    let mut block_sizes = Vec::with_capacity(num_blocks);
    for i in 0..num_blocks {
        let offset = i
            .checked_mul(4)
            .and_then(|value| 16usize.checked_add(value))
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "GCPF: Block table offset overflow",
                )
            })?;
        let size = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        block_sizes.push(size);
    }

    let mut current_offset = header_size;
    let mut uncompressed_data = Vec::with_capacity(uncompressed_size);

    for (i, size) in block_sizes.into_iter().enumerate() {
        trace!(
            "GCPF decompressing block {}: offset={}, size={}",
            i, current_offset, size
        );
        let block_end = current_offset.checked_add(size).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "GCPF: Block offset overflow",
            )
        })?;
        if block_end > data.len() {
            if current_offset.checked_add(4) == Some(data.len())
                && &data[current_offset..current_offset + 4] == b"GCPF"
            {
                debug!("GCPF footer magic reached, stopping decompression cleanly");
                break;
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "GCPF: Block data truncated",
            ));
        }
        let compressed_block = &data[current_offset..block_end];
        current_offset = block_end;

        if !matches!(comp_mode, 0 | 2) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("GCPF: Unsupported compression mode {}", comp_mode),
            ));
        }
        let remaining = uncompressed_size
            .checked_sub(uncompressed_data.len())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "GCPF: Decompressed size overflow",
                )
            })?;
        let expected_block_size = remaining.min(block_size);
        let decoder = zstd::Decoder::new(compressed_block)?;
        let mut decompressed_block = Vec::with_capacity(expected_block_size);
        decoder
            .take(expected_block_size as u64 + 1)
            .read_to_end(&mut decompressed_block)?;
        if decompressed_block.len() != expected_block_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "GCPF: Block decompressed to an unexpected size",
            ));
        }
        uncompressed_data
            .len()
            .checked_add(decompressed_block.len())
            .filter(|total| *total <= uncompressed_size)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "GCPF: Total decompressed size exceeds declared size",
                )
            })?;
        uncompressed_data.extend_from_slice(&decompressed_block);
    }

    if uncompressed_data.len() != uncompressed_size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "GCPF: Decompressed data is truncated",
        ));
    }

    Ok(uncompressed_data)
}

fn collect_bricks_recursive(
    nodes: &[GodotVariant],
    parent_transform: Transform,
    bricks: &mut Vec<VrtxBrick>,
) {
    for node in nodes {
        if let GodotVariant::Array(data) = node {
            if data.len() < 10 {
                continue;
            }

            let name = match &data[0] {
                GodotVariant::String(s) => s.clone(),
                _ => "Part".to_string(),
            };

            let local_pos = match &data[1] {
                GodotVariant::Vector3(v) => *v,
                _ => Vec3::ZERO,
            };

            let local_rot = match &data[2] {
                GodotVariant::Vector3(v) => Quat::from_euler(EulerRot::YXZ, v.y, v.x, v.z),
                _ => Quat::IDENTITY,
            };

            let local_scale = match &data[3] {
                GodotVariant::Vector3(v) => *v,
                _ => Vec3::ONE,
            };

            let color = match &data[4] {
                GodotVariant::Color(c) => *c,
                _ => Color::WHITE,
            };

            let physics_enabled = match &data[6] {
                GodotVariant::Bool(b) => *b,
                _ => false,
            };

            let bounciness = match &data[7] {
                GodotVariant::Float(f) => *f as f32,
                GodotVariant::Int(i) => *i as f32,
                _ => 0.3,
            };

            let shape_type = if data.len() > 10 {
                match &data[10] {
                    GodotVariant::String(s) => s.as_str(),
                    _ => "Cube",
                }
            } else {
                "Cube"
            };

            let shape = if shape_type == "Sphere" {
                crate::common::game::bricks::components::BrickShape::Sphere
            } else {
                crate::common::game::bricks::components::BrickShape::Block
            };

            let is_standard_brick = data.len() >= 15
                && match &data[0] {
                    GodotVariant::String(s) => {
                        s != "NPC"
                            && s != "UIImage"
                            && s != "UIButton"
                            && s != "UIText"
                            && s != "Decal"
                            && s != "RemoteEvent"
                            && s != "Terrain"
                            && s != "Model"
                            && s != "Weld"
                            && s != "Hinge"
                            && s != "Label3D"
                            && s != "Sound"
                            && s != "Script"
                            && s != "LocalScript"
                    }
                    _ => false,
                };

            let bevy_scale = if is_standard_brick {
                match shape {
                    crate::common::game::bricks::components::BrickShape::Block => Vec3::new(
                        local_scale.x / 4.0,
                        local_scale.y / 1.0,
                        local_scale.z / 2.0,
                    ),
                    crate::common::game::bricks::components::BrickShape::Sphere => {
                        local_scale / 2.0
                    }
                }
            } else {
                local_scale
            };

            let local_transform = Transform {
                translation: local_pos,
                rotation: local_rot,
                scale: bevy_scale,
            };

            let global_translation = parent_transform.translation
                + parent_transform
                    .rotation
                    .mul_vec3(local_transform.translation * 0.28);
            let global_rotation = parent_transform.rotation * local_transform.rotation;
            let global_scale = parent_transform.scale * local_transform.scale;

            let global_transform = Transform {
                translation: global_translation,
                rotation: global_rotation,
                scale: global_scale,
            };

            if is_standard_brick {
                bricks.push(VrtxBrick {
                    node_id: bricks.len() as u64,
                    parent_node_id: None,
                    name,
                    transform: global_transform,
                    shape,
                    color,
                    physics_enabled,
                    bounciness,
                    player_can_collide: true,
                    friction: 0.3,
                    gravity_scale: 1.0,
                    mass: 1.0,
                });
            }

            let is_custom_node = match &data[0] {
                GodotVariant::String(s) => {
                    s == "RemoteEvent"
                        || s == "Terrain"
                        || s == "NPC"
                        || s == "Model"
                        || s == "Weld"
                        || s == "Decal"
                        || s == "Terrain"
                        || s == "Hinge"
                        || s == "Label3D"
                        || s == "Sound"
                        || s == "Script"
                        || s == "LocalScript"
                }
                _ => false,
            };

            let children_var = if is_custom_node {
                match &data[0] {
                    GodotVariant::String(s) => {
                        if s == "Model" {
                            data.get(5)
                        } else if s == "Script" || s == "LocalScript" {
                            data.get(4)
                        } else if s == "Weld" {
                            data.get(8)
                        } else if s == "Decal" || s == "Terrain" {
                            data.get(7)
                        } else if s == "NPC" {
                            data.get(16)
                        } else if s == "Hinge" || s == "Label3D" || s == "Sound" {
                            data.get(10)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            } else {
                data.get(9)
            };

            if let Some(GodotVariant::Array(children)) = children_var {
                collect_bricks_recursive(children, global_transform, bricks);
            }
        }
    }
}

fn parse_godot_vrtx(decompressed: &[u8]) -> std::io::Result<VrtxFileState> {
    debug!(
        "Parsing Godot VRTX, decompressed length={}",
        decompressed.len()
    );
    if decompressed.len() >= 4 {
        let first_u32 = u32::from_le_bytes([
            decompressed[0],
            decompressed[1],
            decompressed[2],
            decompressed[3],
        ]);
        debug!(
            "First 4 bytes of decompressed payload: {} (0x{:X})",
            first_u32, first_u32
        );
    }

    let mut parser = GodotParser::new(decompressed);

    if decompressed.len() >= 8 {
        let prefix = u32::from_le_bytes([
            decompressed[0],
            decompressed[1],
            decompressed[2],
            decompressed[3],
        ]) as usize;
        if prefix == decompressed.len() - 4 {
            debug!(
                "Detected Godot store_var length prefix: {} bytes. Skipping prefix.",
                prefix
            );
            parser.offset = 4;
        }
    }

    let variant = parser.parse_variant()?;
    if let GodotVariant::Dictionary(dict) = variant {
        let version = match dict.get("v") {
            Some(GodotVariant::Int(v)) => *v as u32,
            _ => 0,
        };

        let gravity = Vec3::new(0.0, -186.9 * 0.28, 0.0);

        let mut bricks = Vec::new();
        if let Some(GodotVariant::Array(nodes)) = dict.get("n") {
            collect_bricks_recursive(nodes, Transform::IDENTITY, &mut bricks);
        }

        let settings = VrtxSettings {
            ssao: false,
            contact_shadows: false,
            bloom: true,
            ..default()
        };

        let camera_transform =
            Transform::from_xyz(-10.0, 10.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y);

        debug!(
            "Parsing complete: version={}, bricks={}",
            version,
            bricks.len()
        );
        Ok(VrtxFileState {
            version,
            gravity,
            settings,
            lighting: default(),
            camera_transform,
            bricks,
            scripts: Vec::new(),
        })
    } else {
        error!("Parsing failed: Root element is not a Godot dictionary");
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Root element is not a Godot dictionary",
        ))
    }
}

impl VrtxFileState {
    const MAX_FILE_SIZE: u64 = 256 * 1024 * 1024;
    const MAX_BRICK_COUNT: u32 = 100_000;
    const MAX_SCRIPT_COUNT: u32 = 100_000;
    const MAX_NAME_LENGTH: usize = 256;
    const MAX_CODE_LENGTH: usize = 1_048_576;
    const MAX_VRTX_VERSION: u32 = 7;

    fn validate_for_save(&self) -> std::io::Result<()> {
        let invalid = |message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message);
        if self.version != CURRENT_VRTX_VERSION {
            return Err(invalid("New saves must use the current VRTX version"));
        }
        let brick_count = u32::try_from(self.bricks.len())
            .map_err(|_| invalid("Brick count cannot be represented in VRTX"))?;
        let script_count = u32::try_from(self.scripts.len())
            .map_err(|_| invalid("Script count cannot be represented in VRTX"))?;
        if brick_count > Self::MAX_BRICK_COUNT || script_count > Self::MAX_SCRIPT_COUNT {
            return Err(invalid("Project item count exceeds maximum allowed"));
        }
        for brick in &self.bricks {
            if brick.name.len() > Self::MAX_NAME_LENGTH || u16::try_from(brick.name.len()).is_err()
            {
                return Err(invalid("Brick name exceeds maximum allowed length"));
            }
        }
        for script in &self.scripts {
            if script.name.len() > Self::MAX_NAME_LENGTH
                || u16::try_from(script.name.len()).is_err()
            {
                return Err(invalid("Script name exceeds maximum allowed length"));
            }
            if script.code.len() > Self::MAX_CODE_LENGTH
                || u32::try_from(script.code.len()).is_err()
            {
                return Err(invalid("Script code exceeds maximum allowed length"));
            }
            if script.parent_name.as_ref().is_some_and(|name| {
                name.len() > Self::MAX_NAME_LENGTH || u16::try_from(name.len()).is_err()
            }) {
                return Err(invalid("Script parent name exceeds maximum allowed length"));
            }
        }

        let parents: std::collections::HashMap<_, _> = self
            .bricks
            .iter()
            .map(|item| (item.node_id, item.parent_node_id))
            .chain(
                self.scripts
                    .iter()
                    .map(|item| (item.node_id, item.parent_node_id)),
            )
            .collect();
        if parents.len() != self.bricks.len() + self.scripts.len() {
            return Err(invalid("Duplicate VRTX node ID"));
        }
        for (&id, &parent) in &parents {
            if parent == Some(id) {
                return Err(invalid("VRTX node cannot parent itself"));
            }
            if parent.is_some_and(|parent| !parents.contains_key(&parent)) {
                return Err(invalid("VRTX node references a missing parent"));
            }
            let mut seen = std::collections::HashSet::new();
            let mut current = Some(id);
            while let Some(node) = current {
                if !seen.insert(node) {
                    return Err(invalid("VRTX hierarchy contains a cycle"));
                }
                current = parents.get(&node).copied().flatten();
            }
        }
        Ok(())
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        self.validate_for_save()?;
        let destination = std::path::Path::new(path);
        let parent = destination
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."));
        let mut temp_file = tempfile::Builder::new()
            .prefix(".vrtx-")
            .suffix(".tmp")
            .tempfile_in(parent)?;
        let mut writer = BufWriter::new(temp_file.as_file_mut());

        writer.write_all(b"VRTX")?;
        writer.write_all(&self.version.to_le_bytes())?;

        writer.write_all(&self.gravity.x.to_le_bytes())?;
        writer.write_all(&self.gravity.y.to_le_bytes())?;
        writer.write_all(&self.gravity.z.to_le_bytes())?;

        writer.write_all(&[
            if self.settings.ssao { 1 } else { 0 },
            if self.settings.contact_shadows { 1 } else { 0 },
            if self.settings.bloom { 1 } else { 0 },
        ])?;

        if self.version >= 7 {
            writer.write_all(&[
                self.settings.ssao_quality.as_u8(),
                self.settings.shadow_quality.as_u8(),
                if self.settings.soft_shadows { 1 } else { 0 },
                self.settings.anti_aliasing.as_u8(),
            ])?;
            writer.write_all(&self.settings.contact_shadow_length.to_le_bytes())?;
            writer.write_all(&self.settings.bloom_intensity.to_le_bytes())?;
            writer.write_all(&self.settings.exposure_ev100.to_le_bytes())?;

            writer.write_all(&self.lighting.time_of_day.to_le_bytes())?;
            writer.write_all(&self.lighting.sun_intensity.to_le_bytes())?;
            writer.write_all(&self.lighting.ambient_intensity.to_le_bytes())?;
            writer.write_all(&[
                if self.lighting.shadows_enabled { 1 } else { 0 },
                if self.lighting.fog_enabled { 1 } else { 0 },
            ])?;
            writer.write_all(&self.lighting.fog_density.to_le_bytes())?;
            for value in self.lighting.sun_tint.to_srgba().to_f32_array() {
                writer.write_all(&value.to_le_bytes())?;
            }
            for value in self.lighting.ambient_tint.to_srgba().to_f32_array() {
                writer.write_all(&value.to_le_bytes())?;
            }
        }

        writer.write_all(&self.camera_transform.translation.x.to_le_bytes())?;
        writer.write_all(&self.camera_transform.translation.y.to_le_bytes())?;
        writer.write_all(&self.camera_transform.translation.z.to_le_bytes())?;

        writer.write_all(&self.camera_transform.rotation.x.to_le_bytes())?;
        writer.write_all(&self.camera_transform.rotation.y.to_le_bytes())?;
        writer.write_all(&self.camera_transform.rotation.z.to_le_bytes())?;
        writer.write_all(&self.camera_transform.rotation.w.to_le_bytes())?;

        writer.write_all(&(self.bricks.len() as u32).to_le_bytes())?;

        for brick in &self.bricks {
            let name_bytes = brick.name.as_bytes();
            writer.write_all(&(name_bytes.len() as u16).to_le_bytes())?;
            writer.write_all(name_bytes)?;
            if self.version >= 6 {
                writer.write_all(&brick.node_id.to_le_bytes())?;
                writer.write_all(&brick.parent_node_id.unwrap_or(u64::MAX).to_le_bytes())?;
            }

            writer.write_all(&brick.transform.translation.x.to_le_bytes())?;
            writer.write_all(&brick.transform.translation.y.to_le_bytes())?;
            writer.write_all(&brick.transform.translation.z.to_le_bytes())?;

            writer.write_all(&brick.transform.rotation.x.to_le_bytes())?;
            writer.write_all(&brick.transform.rotation.y.to_le_bytes())?;
            writer.write_all(&brick.transform.rotation.z.to_le_bytes())?;
            writer.write_all(&brick.transform.rotation.w.to_le_bytes())?;

            writer.write_all(&brick.transform.scale.x.to_le_bytes())?;
            writer.write_all(&brick.transform.scale.y.to_le_bytes())?;
            writer.write_all(&brick.transform.scale.z.to_le_bytes())?;

            let shape_val = match brick.shape {
                crate::common::game::bricks::components::BrickShape::Block => 0u8,
                crate::common::game::bricks::components::BrickShape::Sphere => 1u8,
            };
            writer.write_all(&[shape_val])?;

            let srgba = brick.color.to_srgba();
            writer.write_all(&srgba.red.to_le_bytes())?;
            writer.write_all(&srgba.green.to_le_bytes())?;
            writer.write_all(&srgba.blue.to_le_bytes())?;
            writer.write_all(&srgba.alpha.to_le_bytes())?;

            writer.write_all(&[if brick.physics_enabled { 1 } else { 0 }])?;
            writer.write_all(&brick.bounciness.to_le_bytes())?;
            writer.write_all(&[if brick.player_can_collide { 1 } else { 0 }])?;
            writer.write_all(&brick.friction.to_le_bytes())?;
            writer.write_all(&brick.gravity_scale.to_le_bytes())?;
            writer.write_all(&brick.mass.to_le_bytes())?;
        }

        if self.version >= 4 {
            writer.write_all(&(self.scripts.len() as u32).to_le_bytes())?;
            for script in &self.scripts {
                let name_bytes = script.name.as_bytes();
                writer.write_all(&(name_bytes.len() as u16).to_le_bytes())?;
                writer.write_all(name_bytes)?;
                if self.version >= 6 {
                    writer.write_all(&script.node_id.to_le_bytes())?;
                    writer.write_all(&script.parent_node_id.unwrap_or(u64::MAX).to_le_bytes())?;
                }

                writer.write_all(&[script.script_type])?;

                let code_bytes = script.code.as_bytes();
                writer.write_all(&(code_bytes.len() as u32).to_le_bytes())?;
                writer.write_all(code_bytes)?;

                if let Some(ref parent) = script.parent_name {
                    let p_bytes = parent.as_bytes();
                    writer.write_all(&(p_bytes.len() as u16).to_le_bytes())?;
                    writer.write_all(p_bytes)?;
                } else {
                    writer.write_all(&0u16.to_le_bytes())?;
                }

                if self.version >= 5 {
                    writer.write_all(&[if script.enabled { 1 } else { 0 }])?;
                }
            }
        }

        writer.flush()?;
        writer.get_ref().sync_all()?;
        drop(writer);
        temp_file
            .persist(destination)
            .map_err(|error| error.error)?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        debug!("load_from_file: Attempting to open file: {}", path);
        let data = read_file_with_limit(path, Self::MAX_FILE_SIZE)?;
        debug!("load_from_file: Read {} bytes from {}", data.len(), path);

        if data.len() >= 4 && &data[0..4] == b"VRTX" {
            let mut reader = BufReader::new(&data[4..]);
            let mut version_bytes = [0u8; 4];
            reader.read_exact(&mut version_bytes)?;
            let version = u32::from_le_bytes(version_bytes);
            if version > Self::MAX_VRTX_VERSION {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Unsupported .VRTX file version",
                ));
            }
            debug!("load_from_file: VRTX format version is {}", version);

            let (gravity, settings, lighting, camera_transform, count) = if version >= 1 {
                debug!("load_from_file: Parsing version 1/2/3/4 header");
                let mut gx = [0u8; 4];
                reader.read_exact(&mut gx)?;
                let mut gy = [0u8; 4];
                reader.read_exact(&mut gy)?;
                let mut gz = [0u8; 4];
                reader.read_exact(&mut gz)?;
                let gravity = Vec3::new(
                    f32::from_le_bytes(gx),
                    f32::from_le_bytes(gy),
                    f32::from_le_bytes(gz),
                );

                let mut settings_bytes = [0u8; 3];
                reader.read_exact(&mut settings_bytes)?;
                let mut settings = VrtxSettings {
                    ssao: settings_bytes[0] != 0,
                    contact_shadows: settings_bytes[1] != 0,
                    bloom: settings_bytes[2] != 0,
                    ..default()
                };
                let mut lighting =
                    crate::common::game::environment::lighting::LightingService::default();
                if version >= 7 {
                    let mut quality_bytes = [0; 4];
                    reader.read_exact(&mut quality_bytes)?;
                    settings.ssao_quality =
                        crate::common::core::performance::AmbientOcclusionQuality::from_u8(
                            quality_bytes[0],
                        );
                    settings.shadow_quality =
                        crate::common::core::performance::ShadowQuality::from_u8(quality_bytes[1]);
                    settings.soft_shadows = quality_bytes[2] != 0;
                    settings.anti_aliasing =
                        crate::common::core::performance::AntiAliasing::from_u8(quality_bytes[3]);
                    settings.contact_shadow_length = read_f32(&mut reader)?;
                    settings.bloom_intensity = read_f32(&mut reader)?;
                    settings.exposure_ev100 = read_f32(&mut reader)?;

                    lighting.time_of_day = read_f32(&mut reader)?;
                    lighting.sun_intensity = read_f32(&mut reader)?;
                    lighting.ambient_intensity = read_f32(&mut reader)?;
                    let mut lighting_flags = [0; 2];
                    reader.read_exact(&mut lighting_flags)?;
                    lighting.shadows_enabled = lighting_flags[0] != 0;
                    lighting.fog_enabled = lighting_flags[1] != 0;
                    lighting.fog_density = read_f32(&mut reader)?;
                    lighting.sun_tint = Color::srgba(
                        read_f32(&mut reader)?,
                        read_f32(&mut reader)?,
                        read_f32(&mut reader)?,
                        read_f32(&mut reader)?,
                    );
                    lighting.ambient_tint = Color::srgba(
                        read_f32(&mut reader)?,
                        read_f32(&mut reader)?,
                        read_f32(&mut reader)?,
                        read_f32(&mut reader)?,
                    );
                }

                let mut cx = [0u8; 4];
                reader.read_exact(&mut cx)?;
                let mut cy = [0u8; 4];
                reader.read_exact(&mut cy)?;
                let mut cz = [0u8; 4];
                reader.read_exact(&mut cz)?;
                let camera_translation = Vec3::new(
                    f32::from_le_bytes(cx),
                    f32::from_le_bytes(cy),
                    f32::from_le_bytes(cz),
                );

                let mut crx = [0u8; 4];
                reader.read_exact(&mut crx)?;
                let mut cry = [0u8; 4];
                reader.read_exact(&mut cry)?;
                let mut crz = [0u8; 4];
                reader.read_exact(&mut crz)?;
                let mut crw = [0u8; 4];
                reader.read_exact(&mut crw)?;
                let camera_rotation = Quat::from_xyzw(
                    f32::from_le_bytes(crx),
                    f32::from_le_bytes(cry),
                    f32::from_le_bytes(crz),
                    f32::from_le_bytes(crw),
                );

                let camera_transform = Transform {
                    translation: camera_translation,
                    rotation: camera_rotation,
                    scale: Vec3::ONE,
                };

                let mut count_bytes = [0u8; 4];
                reader.read_exact(&mut count_bytes)?;
                let count = u32::from_le_bytes(count_bytes);

                (gravity, settings, lighting, camera_transform, count)
            } else if version == 0 {
                debug!("load_from_file: Parsing version 0 header");
                let mut gx = [0u8; 4];
                reader.read_exact(&mut gx)?;
                let mut gy = [0u8; 4];
                reader.read_exact(&mut gy)?;
                let mut gz = [0u8; 4];
                reader.read_exact(&mut gz)?;
                let gravity = Vec3::new(
                    f32::from_le_bytes(gx),
                    f32::from_le_bytes(gy),
                    f32::from_le_bytes(gz),
                );

                let settings = VrtxSettings {
                    ssao: false,
                    contact_shadows: false,
                    bloom: true,
                    ..default()
                };
                let lighting = default();

                let camera_transform =
                    Transform::from_xyz(-10.0, 10.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y);

                let mut count_bytes = [0u8; 4];
                reader.read_exact(&mut count_bytes)?;
                let count = u32::from_le_bytes(count_bytes);

                (gravity, settings, lighting, camera_transform, count)
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Unsupported .VRTX file version",
                ));
            };

            if count > Self::MAX_BRICK_COUNT {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Brick count exceeds maximum allowed",
                ));
            }

            debug!("load_from_file: Expecting {} bricks", count);
            let mut bricks = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let mut name_len_bytes = [0u8; 2];
                reader.read_exact(&mut name_len_bytes)?;
                let name_len = u16::from_le_bytes(name_len_bytes) as usize;
                if name_len > Self::MAX_NAME_LENGTH {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Brick name exceeds maximum allowed length",
                    ));
                }
                let mut name_bytes = vec![0u8; name_len];
                reader.read_exact(&mut name_bytes)?;
                let name = String::from_utf8(name_bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                let (node_id, parent_node_id) = if version >= 6 {
                    let mut id = [0u8; 8];
                    reader.read_exact(&mut id)?;
                    let mut parent = [0u8; 8];
                    reader.read_exact(&mut parent)?;
                    let parent = u64::from_le_bytes(parent);
                    (
                        u64::from_le_bytes(id),
                        (parent != u64::MAX).then_some(parent),
                    )
                } else {
                    (bricks.len() as u64, None)
                };

                let mut tx = [0u8; 4];
                reader.read_exact(&mut tx)?;
                let mut ty = [0u8; 4];
                reader.read_exact(&mut ty)?;
                let mut tz = [0u8; 4];
                reader.read_exact(&mut tz)?;
                let translation = Vec3::new(
                    f32::from_le_bytes(tx),
                    f32::from_le_bytes(ty),
                    f32::from_le_bytes(tz),
                );

                let mut rx = [0u8; 4];
                reader.read_exact(&mut rx)?;
                let mut ry = [0u8; 4];
                reader.read_exact(&mut ry)?;
                let mut rz = [0u8; 4];
                reader.read_exact(&mut rz)?;
                let mut rw = [0u8; 4];
                reader.read_exact(&mut rw)?;
                let rotation = Quat::from_xyzw(
                    f32::from_le_bytes(rx),
                    f32::from_le_bytes(ry),
                    f32::from_le_bytes(rz),
                    f32::from_le_bytes(rw),
                );

                let mut sx = [0u8; 4];
                reader.read_exact(&mut sx)?;
                let mut sy = [0u8; 4];
                reader.read_exact(&mut sy)?;
                let mut sz = [0u8; 4];
                reader.read_exact(&mut sz)?;
                let scale = Vec3::new(
                    f32::from_le_bytes(sx),
                    f32::from_le_bytes(sy),
                    f32::from_le_bytes(sz),
                );

                let transform = Transform {
                    translation,
                    rotation,
                    scale,
                };

                let mut shape_bytes = [0u8; 1];
                reader.read_exact(&mut shape_bytes)?;
                let shape = match shape_bytes[0] {
                    0 => crate::common::game::bricks::components::BrickShape::Block,
                    1 => crate::common::game::bricks::components::BrickShape::Sphere,
                    _ => {
                        error!("load_from_file: Invalid brick shape enum value");
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Invalid brick shape enum value",
                        ));
                    }
                };

                let mut cr = [0u8; 4];
                reader.read_exact(&mut cr)?;
                let mut cg = [0u8; 4];
                reader.read_exact(&mut cg)?;
                let mut cb = [0u8; 4];
                reader.read_exact(&mut cb)?;
                let mut ca = [0u8; 4];
                reader.read_exact(&mut ca)?;
                let color = Color::Srgba(Srgba::new(
                    f32::from_le_bytes(cr),
                    f32::from_le_bytes(cg),
                    f32::from_le_bytes(cb),
                    f32::from_le_bytes(ca),
                ));

                let mut phys_enabled_bytes = [0u8; 1];
                reader.read_exact(&mut phys_enabled_bytes)?;
                let physics_enabled = phys_enabled_bytes[0] != 0;

                let mut bounciness_bytes = [0u8; 4];
                reader.read_exact(&mut bounciness_bytes)?;
                let bounciness = f32::from_le_bytes(bounciness_bytes);

                let player_can_collide = if version >= 2 {
                    let mut player_can_collide_bytes = [0u8; 1];
                    reader.read_exact(&mut player_can_collide_bytes)?;
                    player_can_collide_bytes[0] != 0
                } else {
                    true
                };

                let (friction, gravity_scale, mass) = if version >= 3 {
                    let mut friction_bytes = [0u8; 4];
                    reader.read_exact(&mut friction_bytes)?;
                    let mut gravity_scale_bytes = [0u8; 4];
                    reader.read_exact(&mut gravity_scale_bytes)?;
                    let mut mass_bytes = [0u8; 4];
                    reader.read_exact(&mut mass_bytes)?;
                    (
                        f32::from_le_bytes(friction_bytes),
                        f32::from_le_bytes(gravity_scale_bytes),
                        f32::from_le_bytes(mass_bytes),
                    )
                } else {
                    (0.3, 1.0, 1.0)
                };

                bricks.push(VrtxBrick {
                    node_id,
                    parent_node_id,
                    name,
                    transform,
                    shape,
                    color,
                    physics_enabled,
                    bounciness,
                    player_can_collide,
                    friction,
                    gravity_scale,
                    mass,
                });
            }

            let mut scripts = Vec::new();
            if version >= 4 {
                let mut script_count_bytes = [0u8; 4];
                reader.read_exact(&mut script_count_bytes)?;
                let script_count = u32::from_le_bytes(script_count_bytes);
                if script_count > Self::MAX_SCRIPT_COUNT {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Script count exceeds maximum allowed",
                    ));
                }
                for _ in 0..script_count {
                    let mut name_len_bytes = [0u8; 2];
                    reader.read_exact(&mut name_len_bytes)?;
                    let name_len = u16::from_le_bytes(name_len_bytes) as usize;
                    if name_len > Self::MAX_NAME_LENGTH {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Script name exceeds maximum allowed length",
                        ));
                    }
                    let mut name_bytes = vec![0u8; name_len];
                    reader.read_exact(&mut name_bytes)?;
                    let name = String::from_utf8(name_bytes).unwrap_or_default();
                    let (node_id, parent_node_id) = if version >= 6 {
                        let mut id = [0u8; 8];
                        reader.read_exact(&mut id)?;
                        let mut parent = [0u8; 8];
                        reader.read_exact(&mut parent)?;
                        let parent = u64::from_le_bytes(parent);
                        (
                            u64::from_le_bytes(id),
                            (parent != u64::MAX).then_some(parent),
                        )
                    } else {
                        ((bricks.len() + scripts.len()) as u64, None)
                    };

                    let mut type_bytes = [0u8; 1];
                    reader.read_exact(&mut type_bytes)?;
                    let script_type = type_bytes[0];

                    let mut code_len_bytes = [0u8; 4];
                    reader.read_exact(&mut code_len_bytes)?;
                    let code_len = u32::from_le_bytes(code_len_bytes) as usize;
                    if code_len > Self::MAX_CODE_LENGTH {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Script code exceeds maximum allowed length",
                        ));
                    }
                    let mut code_bytes = vec![0u8; code_len];
                    reader.read_exact(&mut code_bytes)?;
                    let code = String::from_utf8(code_bytes).unwrap_or_default();

                    let mut p_len_bytes = [0u8; 2];
                    reader.read_exact(&mut p_len_bytes)?;
                    let p_len = u16::from_le_bytes(p_len_bytes) as usize;
                    if p_len > Self::MAX_NAME_LENGTH {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Script parent name exceeds maximum allowed length",
                        ));
                    }
                    let parent_name = if p_len > 0 {
                        let mut p_bytes = vec![0u8; p_len];
                        reader.read_exact(&mut p_bytes)?;
                        Some(String::from_utf8(p_bytes).unwrap_or_default())
                    } else {
                        None
                    };

                    let enabled = if version >= 5 {
                        let mut enabled_byte = [0u8; 1];
                        reader.read_exact(&mut enabled_byte)?;
                        enabled_byte[0] != 0
                    } else {
                        true
                    };

                    scripts.push(VrtxScript {
                        node_id,
                        parent_node_id,
                        name,
                        script_type,
                        code,
                        parent_name,
                        enabled,
                    });
                }
            }

            debug!(
                "load_from_file: Successfully parsed {} bricks and {} scripts from standard VRTX file",
                bricks.len(),
                scripts.len()
            );
            Ok(Self {
                version,
                gravity,
                settings,
                lighting,
                camera_transform,
                bricks,
                scripts,
            })
        } else if data.len() >= 4 && &data[0..4] == b"GCPF" {
            debug!("load_from_file: Detected legacy GCPF (Godot) file format");
            let decompressed = decompress_gcpf_file(&data)?;
            debug!(
                "load_from_file: Successfully decompressed GCPF file into {} bytes",
                decompressed.len()
            );
            let parsed_state = parse_godot_vrtx(&decompressed)?;
            debug!(
                "load_from_file: Successfully parsed Godot VRTX map with {} bricks",
                parsed_state.bricks.len()
            );
            Ok(parsed_state)
        } else {
            error!("load_from_file: Unknown or invalid file signature");
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unknown or invalid file signature",
            ))
        }
    }
}

#[cfg(test)]
mod hierarchy_tests {
    use super::*;

    #[test]
    fn current_version_round_trips_project_state() {
        let path = std::env::temp_dir().join(format!("rave-vrtx-v7-{}.vrtx", std::process::id()));
        let state = VrtxFileState {
            version: CURRENT_VRTX_VERSION,
            gravity: Vec3::new(0.0, -9.8, 0.0),
            settings: VrtxSettings {
                ssao: true,
                ssao_quality: crate::common::core::performance::AmbientOcclusionQuality::Ultra,
                contact_shadows: false,
                contact_shadow_length: 0.8,
                bloom: true,
                bloom_intensity: 0.12,
                ..default()
            },
            lighting: crate::common::game::environment::lighting::LightingService {
                time_of_day: 17.5,
                sun_intensity: 1.4,
                fog_density: 0.7,
                ..default()
            },
            camera_transform: Transform::IDENTITY,
            bricks: vec![VrtxBrick {
                node_id: 10,
                parent_node_id: None,
                name: "Duplicate".into(),
                transform: Transform::IDENTITY,
                shape: crate::common::game::bricks::components::BrickShape::Block,
                color: Color::WHITE,
                physics_enabled: true,
                bounciness: 0.3,
                player_can_collide: true,
                friction: 0.3,
                gravity_scale: 1.0,
                mass: 1.0,
            }],
            scripts: vec![VrtxScript {
                node_id: 11,
                parent_node_id: Some(10),
                name: "Duplicate".into(),
                script_type: 0,
                code: "print('ok')".into(),
                parent_name: None,
                enabled: true,
            }],
        };
        state.save_to_file(path.to_str().unwrap()).unwrap();
        let loaded = VrtxFileState::load_from_file(path.to_str().unwrap()).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(loaded.version, CURRENT_VRTX_VERSION);
        assert_eq!(loaded.bricks[0].node_id, 10);
        assert_eq!(loaded.scripts[0].parent_node_id, Some(10));
        assert_eq!(loaded.bricks[0].name, loaded.scripts[0].name);
        assert_eq!(loaded.lighting.time_of_day, 17.5);
        assert_eq!(loaded.lighting.sun_intensity, 1.4);
        assert_eq!(loaded.lighting.fog_density, 0.7);
        assert_eq!(
            loaded.settings.ssao_quality,
            crate::common::core::performance::AmbientOcclusionQuality::Ultra
        );
        assert_eq!(loaded.settings.contact_shadow_length, 0.8);
        assert_eq!(loaded.settings.bloom_intensity, 0.12);
    }

    #[test]
    fn atomic_save_overwrites_an_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("project.vrtx");
        std::fs::write(&path, b"existing contents").unwrap();
        let mut state = VrtxFileState {
            version: CURRENT_VRTX_VERSION,
            gravity: Vec3::new(1.0, 2.0, 3.0),
            settings: VrtxSettings::default(),
            lighting: default(),
            camera_transform: Transform::IDENTITY,
            bricks: Vec::new(),
            scripts: Vec::new(),
        };

        state.save_to_file(path.to_str().unwrap()).unwrap();
        state.gravity = Vec3::new(4.0, 5.0, 6.0);
        state.save_to_file(path.to_str().unwrap()).unwrap();

        let loaded = VrtxFileState::load_from_file(path.to_str().unwrap()).unwrap();
        assert_eq!(loaded.gravity, state.gravity);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn rejects_oversized_gcpf_declaration_before_allocating() {
        let mut data = Vec::from(&b"GCPF"[..]);
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&4096u32.to_le_bytes());
        data.extend_from_slice(&((VrtxFileState::MAX_FILE_SIZE + 1) as u32).to_le_bytes());
        assert_eq!(
            decompress_gcpf_file(&data).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn rejects_gcpf_block_output_larger_than_declared() {
        let compressed = zstd::encode_all(&b"ab"[..], 0).unwrap();
        let mut data = Vec::from(&b"GCPF"[..]);
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        data.extend_from_slice(&compressed);

        assert_eq!(
            decompress_gcpf_file(&data).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn bounded_reader_rejects_oversized_input() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("oversized.vrtx");
        std::fs::write(&path, [0u8; 9]).unwrap();
        assert_eq!(
            read_file_with_limit(path.to_str().unwrap(), 8)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn version_four_and_later_require_script_count() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("truncated.vrtx");
        let state = VrtxFileState {
            version: CURRENT_VRTX_VERSION,
            gravity: Vec3::ZERO,
            settings: VrtxSettings::default(),
            lighting: default(),
            camera_transform: Transform::IDENTITY,
            bricks: Vec::new(),
            scripts: Vec::new(),
        };
        state.save_to_file(path.to_str().unwrap()).unwrap();
        let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_len(file.metadata().unwrap().len() - 4).unwrap();

        assert_eq!(
            VrtxFileState::load_from_file(path.to_str().unwrap())
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::UnexpectedEof
        );
    }
}
