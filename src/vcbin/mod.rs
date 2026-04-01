use crate::error::{Result, VenturiError};
use crate::vm::bytecode::Instruction;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MAGIC: &[u8; 4] = b"VCBN";
const VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcBinMeta {
    pub name: String,
    pub version: String,
    pub timestamp: u64,
    pub mode: String, // "plane" or "vortex"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortDef {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcBinInterface {
    pub inputs: Vec<PortDef>,
    pub outputs: Vec<PortDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcBinPermissions {
    pub allowed_vans: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcBinGraph {
    pub nodes: Vec<String>,
    pub edges: Vec<GraphEdge>,
    pub entry: String,
    pub exit: String,
}

#[derive(Debug, Clone)]
pub struct VcBin {
    pub metadata: VcBinMeta,
    pub interface: VcBinInterface,
    pub permissions: VcBinPermissions,
    pub graph: VcBinGraph,
    pub bytecode: Vec<Instruction>,
    pub hash: [u8; 32],
}

impl VcBin {
    pub fn new(
        name: String,
        mode: String,
        interface: VcBinInterface,
        permissions: VcBinPermissions,
        graph: VcBinGraph,
        bytecode: Vec<Instruction>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut vcbin = VcBin {
            metadata: VcBinMeta {
                name,
                version: "1.0".to_string(),
                timestamp,
                mode,
            },
            interface,
            permissions,
            graph,
            bytecode,
            hash: [0u8; 32],
        };

        vcbin.hash = vcbin.compute_hash();
        vcbin
    }

    fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();

        hasher.update(MAGIC);
        hasher.update(VERSION.to_le_bytes());

        let meta_json = serde_json::to_vec(&self.metadata).unwrap_or_default();
        hasher.update(&meta_json);

        let iface_json = serde_json::to_vec(&self.interface).unwrap_or_default();
        hasher.update(&iface_json);

        let perms_json = serde_json::to_vec(&self.permissions).unwrap_or_default();
        hasher.update(&perms_json);

        let graph_json = serde_json::to_vec(&self.graph).unwrap_or_default();
        hasher.update(&graph_json);

        let bytecode_json = serde_json::to_vec(&self.bytecode).unwrap_or_default();
        hasher.update(&bytecode_json);

        hasher.finalize().into()
    }

    pub fn verify_hash(&self) -> bool {
        let computed = self.compute_hash();
        computed == self.hash
    }

    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let mut file = std::fs::File::create(path)?;

        // Header: magic + version + flags
        file.write_all(MAGIC)?;
        file.write_all(&VERSION.to_le_bytes())?;
        let flags: u16 = 0;
        file.write_all(&flags.to_le_bytes())?;

        // Write JSON sections with length prefixes
        Self::write_section(&mut file, &serde_json::to_vec(&self.metadata)?)?;
        Self::write_section(&mut file, &serde_json::to_vec(&self.interface)?)?;
        Self::write_section(&mut file, &serde_json::to_vec(&self.permissions)?)?;
        Self::write_section(&mut file, &serde_json::to_vec(&self.graph)?)?;
        Self::write_section(&mut file, &serde_json::to_vec(&self.bytecode)?)?;

        // Hash section
        file.write_all(&self.hash)?;

        Ok(())
    }

    fn write_section(file: &mut std::fs::File, data: &[u8]) -> Result<()> {
        let len = data.len() as u64;
        file.write_all(&len.to_le_bytes())?;
        file.write_all(data)?;
        Ok(())
    }

    pub fn read_from_file(path: &Path) -> Result<Self> {
        let mut file = std::fs::File::open(path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Self::from_bytes(&buf)
    }

    fn from_bytes(buf: &[u8]) -> Result<Self> {
        let mut pos = 0;

        // Check magic
        if buf.len() < 8 {
            return Err(VenturiError::VcBin("File too short".to_string()));
        }
        if &buf[0..4] != MAGIC {
            return Err(VenturiError::VcBin("Invalid magic bytes".to_string()));
        }
        pos += 4;

        let _version = u16::from_le_bytes([buf[pos], buf[pos + 1]]);
        pos += 2;
        let _flags = u16::from_le_bytes([buf[pos], buf[pos + 1]]);
        pos += 2;

        let (metadata, consumed) = Self::read_section::<VcBinMeta>(&buf[pos..])?;
        pos += consumed;

        let (interface, consumed) = Self::read_section::<VcBinInterface>(&buf[pos..])?;
        pos += consumed;

        let (permissions, consumed) = Self::read_section::<VcBinPermissions>(&buf[pos..])?;
        pos += consumed;

        let (graph, consumed) = Self::read_section::<VcBinGraph>(&buf[pos..])?;
        pos += consumed;

        let (bytecode, consumed) = Self::read_section::<Vec<Instruction>>(&buf[pos..])?;
        pos += consumed;

        let mut hash = [0u8; 32];
        if pos + 32 <= buf.len() {
            hash.copy_from_slice(&buf[pos..pos + 32]);
        }

        Ok(VcBin {
            metadata,
            interface,
            permissions,
            graph,
            bytecode,
            hash,
        })
    }

    fn read_section<T: for<'de> Deserialize<'de>>(buf: &[u8]) -> Result<(T, usize)> {
        if buf.len() < 8 {
            return Err(VenturiError::VcBin("Section truncated".to_string()));
        }
        let len = u64::from_le_bytes(buf[0..8].try_into().unwrap()) as usize;
        if buf.len() < 8 + len {
            return Err(VenturiError::VcBin("Section data truncated".to_string()));
        }
        let data: T = serde_json::from_slice(&buf[8..8 + len])?;
        Ok((data, 8 + len))
    }
}
