//! References:
//!
//! [McKus84] Marshall K McKusick, William N Joy, Samuel J Leffler,
//! and Robert S Fabry. 1984. ``A Fast FileSystem for Unix''.
//! ACM Transactions on Computer Systems 2, 3 (Aug. 1984),
//! 181-197. https://doi.org/10.1145/989.990

use core::cmp;
use core::fmt::{self, Write};
use core::mem;
use core::ops::Range;
use core::ptr;

use bitflags::bitflags;
use bitstruct::bitstruct;
use static_assertions::const_assert;

/// The size of a "Device Block".  That is, the size of a
/// physical block on the underlying storage device.
pub const DEV_BLOCK_SIZE: usize = 512;

/// Lg(DEV_BLOCK_SIZE)
pub const DEV_BIT_SHIFT: usize = 9;

/// Offset of the boot block, relative to the start of the
/// partition, in bytes.
pub const BOOT_BLOCK_OFFSET: usize = 0;

/// Size of the boot block, in bytes.
pub const BOOT_BLOCK_SIZE: usize = 8192;

/// The offset of the first superblock, in bytes.
pub const SUPER_BLOCK_OFFSET: usize = BOOT_BLOCK_SIZE + BOOT_BLOCK_OFFSET;

/// Size of the super block, in bytes.
pub const SUPER_BLOCK_SIZE: usize = 8192;

/// Number of Bits per Byte
///
/// This is an anachronism, but at the time that FFS was
/// written, there were still machines that used 36 bit words
/// and variable byte sizes and so on.  Peripherals like the
/// DEC RP06, that worked on those computers as well as 8-bit
/// byte-addressed machines, were fairly common.
pub const NBBY: usize = 8;

/// Maximum number of bits in a file size.
pub const FILE_SIZE_BITS: usize = NBBY * core::mem::size_of::<u32>() + DEV_BIT_SHIFT;

/// Maximum offset mask.
pub const MAX_OFFSET: usize = 1 << (FILE_SIZE_BITS - 1) - 1;

/// Maximum mount point length
pub const MAX_MOUNT_LEN: usize = 512;

/// Maximum size of checksum buffers
pub const MAX_CKSUM_BUFS: usize = 32;

/// Maximum logical block size.
pub const _MAX_BLOCK_SIZE: usize = 8192;

/// Maximum number of fragments per block
pub const MAX_FRAG: usize = 8;

/// Per-cylinder group informations ummary.
#[repr(C)]
#[derive(Debug)]
pub struct CylGroupSummary {
    ndir: u32,   // number of directories
    nbfree: u32, // number of free blocks
    nifree: u32, // number of free inodes
    nffree: u32, // number of free fragments
}

/// Whether the cylinder group summary in the superblock should
/// be recalculated.
pub const _SI_OK: u32 = 0b00;
pub const _SI_BAD: u32 = 0b01;

/// Magic number identifying a UFS file system. Kirk's birthday?
pub const MAGIC: u32 = 0x011954;

pub const _MTB_MAGIC: u32 = 0xdecade;

/// An amount of time until a clean filesystem requires a mandatory
/// fsck(8).
pub const _FSOKAY: u32 = 0x7c269d38;

/// Valid states in the `clean` member of the superblock.
#[repr(u8)]
#[derive(Debug)]
pub enum State {
    Active = 0,
    Clean = 1,
    Stable = 2,
    Fix = 0xfc,
    Log = 0xfd,
    Suspend = 0xfe,
    Bad = 0xff,
}

bitflags! {
    /// Supported `flags` in the superblock.

    #[derive(Clone, Copy, Debug)]
    pub struct Flags: u8 {
        const LARGE_FILES = 1;
    }
}

/// Superblock.
///
/// Most disk addresses are in fragments.
/// Note that SVR4 reverses the `nspect` and `state_ts` fields.
#[repr(C)]
#[derive(Debug)]
pub struct SuperBlock {
    link: u32,                       // Linked list of filesystems
    rolled: u32,                     // Logging only: fully rolled?
    sblkno: u32,                     // Addr of syper-block in filesys
    cblkno: u32,                     // Offset of cylinder group in filesys
    iblkno: u32,                     // Offset of inode-blocks in filesys
    dblkno: u32,                     // Offset of first data after cyl group
    cgoffset: u32,                   // Offset of cylinder group in cylinder
    cgmask: u32,                     // Used to calc mod ntrack
    time: u32,                       // Last time written
    size: u32,                       // Number of blocks in filesys
    dsize: u32,                      // Number of data blocks in filesys
    ncg: u32,                        // Number of cylinder groups in filesys
    bsize: u32,                      // Size of "basic" blocks in filesys
    fsize: u32,                      // Size of "fragment" blocks in filesys
    frag: u32,                       // Number of fragments in a block
    minfree: u32,                    // Min percentage of free blocks in filesys
    rotdelay: u32,                   // MS until optimal for next block
    rps: u32,                        // Disk revolutions per second
    bmask: u32,                      // `blkoff`: block offsets
    fmask: u32,                      // `fragoff`: fragment offsets
    bshift: u32,                     // `lblkno`: logical block number
    fshift: u32,                     // `numfrags`: number of fragments
    maxcontig: u32,                  // Max number of contiguous blocks
    maxbpg: u32,                     // Max number of blocks per cyl group
    fragshift: u32,                  // Block to fragment shift
    fsbtodb: u32,                    // FS block <-> dev block shift constant
    sbsize: u32,                     // Actual size of superblock
    csmask: u32,                     // CylGroupSummary block offset
    csshift: u32,                    // CylGroupSummary block number
    nindir: u32,                     // Value of `NINDIR`
    inopb: u32,                      // Value of `INOPB`
    nspf: u32,                       // Value of `NSPF`
    optpref: u32,                    // Optimization preference (space or time)
    state_ts: u32,                   // File system state time stamp
    si_state: u32,                   // Summary info state (logging only?)
    trackskew: u32,                  // Sector 0 skew, per track
    id: [u32; 2],                    // Unique ID for filesystem (unused)
    csaddr: u32,                     // Block address of cylinder group summary area
    cssize: u32,                     // Size of cylinder group summary area
    cgsize: u32,                     // Cylinder group size
    cntrack: u32,                    // Tracks per cylinder
    trknsec: u32,                    // Sectors per track
    cnsec: u32,                      // Sectors per cylinder
    numcyl: u32,                     // Number of cylinders in the file system
    cpg: u32,                        // Cylinders per group
    ipg: u32,                        // inodes per group
    fpg: u32,                        // Fragments per group (num group blocks * frag size)
    cstotal: CylGroupSummary,        // Cylinder summary information
    sb_mod: u8,                      // Superblock modified flag
    clean: u8,                       // Filesystem state flag
    ronly: u8,                       // Mounted read-only
    flags: u8,                       // Bit mask of flags
    mountpt: [u8; MAX_MOUNT_LEN],    // Mount point
    cgrotor: u32,                    // Last cylinder group searched
    _ocksums: [u32; MAX_CKSUM_BUFS], // Old checksum buffers
    cyccyl: u32,                     // Cylinders per cycle
    _oposttbl: [[u16; 16]; 8],       // Old rotation block list heads
    _resv: [u32; 51],                // Reserved
    version: u32,                    // UFS minor version
    logblkno: u32,                   // Block number of embedded log
    reclaim: u32,                    // Reclaim open but deleted files
    _resv1: u32,                     // Reserved
    nspect: u32,                     // Sectors per track, include spares
    qbmask: [u32; 2],                // !fs_bmask for u64
    qfmask: [u32; 2],                // !fs_fmask for u64
    post_tbl_fmt: u32,               // Format of positional layout tables
    num_rot_pos: u32,                // Number of rotational positions
    post_blk_off: u32,               // Short rotation block list head
    rot_blk_off: u32,                // Blocks for each rotation
    magic: u32,                      // Kirk's birthday
}

const_assert!(core::mem::size_of::<SuperBlock>() <= SUPER_BLOCK_SIZE);

impl SuperBlock {
    /// Returns the superblock, as "read" from the given "disk."
    pub fn read(disk: &[u8]) -> SuperBlock {
        let sbb = &disk[SUPER_BLOCK_OFFSET..SUPER_BLOCK_OFFSET + SUPER_BLOCK_SIZE];
        let p = sbb.as_ptr().cast::<SuperBlock>();
        let sb = unsafe { ptr::read_unaligned(p) };
        assert_eq!(sb.magic, MAGIC);
        sb
    }

    /// Returns the block address of the given cylinder group, as
    /// an offset from the beginning of the underlying device.
    pub fn cgbase(&self, cylgrp: u32) -> u32 {
        assert!(cylgrp < self.ncg);
        self.fpg * cylgrp
    }

    /// Returns the block address of the start of the given cylinder
    /// group, as an offset from the beginning of the underlying device.
    /// The start address differs from the base address in that the start
    /// is offset by a multiple, derived from the group number, from the
    /// base address.  This a bit of an historical accident, in that the
    /// physical placement of e.g. the superblock in each cylinder group
    /// was deliberately offset to facilitate recovery in the event of a
    /// head crash or other catastrophic physical drive failure; this
    /// method ensured that all copies of the superblock were not in the
    /// same cylinder or platter, but rather, "spiraled down into the pack"
    /// [McKus84].
    pub fn cgstart(&self, cylgrp: u32) -> u32 {
        self.cgbase(cylgrp) + self.cgoffset * (cylgrp & !self.cgmask)
    }

    /// Returns the start of the inode region for the given cylinder group.
    pub fn cgimin(&self, cylgrp: u32) -> u32 {
        self.cgstart(cylgrp) + self.iblkno
    }

    /// Returns the start of the data region for the given cylinder group.
    pub fn cgdmin(&self, cylgrp: u32) -> u32 {
        self.cgstart(cylgrp) + self.dblkno
    }

    pub fn blkstofrags(&self, blks: u32) -> u32 {
        blks << self.fragshift
    }

    /// Returns the cylinder group number for the given inode number.
    pub fn itog(&self, ino: u32) -> u32 {
        ino / self.ipg
    }

    /// Inode number to disk address.
    pub fn itod(&self, ino: u32) -> u32 {
        self.cgimin(self.itog(ino)) + self.blkstofrags((ino % self.ipg) / self.inopb)
    }

    /// Offset of inode in block.
    pub fn itoo(&self, ino: u32) -> u32 {
        ino % self.inopb
    }

    /// The number of inodes per fragment.
    pub fn inopf(&self) -> u32 {
        self.inopb >> self.fragshift
    }

    /// Returns the offset of given inode, relative to the
    /// start of the storage area.
    pub fn inode_offset(&self, ino: u32) -> usize {
        let ibase = u64::from(self.itod(ino)) * self.fsize as u64;
        let ioff = self.itoo(ino) as usize * mem::size_of::<DInode>();
        ibase as usize + ioff
    }

    /// Returns the logical block number in the file for the given offset.
    pub fn lblkno(&self, off: u64) -> u64 {
        off >> self.bshift
    }

    /// Returns the disk block number of a file system block.
    pub fn fsbtodb(&self, fbno: usize) -> usize {
        fbno << self.fsbtodb as usize
    }

    /// Returns the "clean" state of the filesystem.
    pub fn state(&self) -> Result<State, ()> {
        match self.clean {
            0x00 => Ok(State::Active),
            0x01 => Ok(State::Clean),
            0x02 => Ok(State::Stable),
            0xfc => Ok(State::Fix),
            0xfd => Ok(State::Log),
            0xfe => Ok(State::Suspend),
            0xff => Ok(State::Bad),
            _ => Err(()),
        }
    }

    /// Returns the "Flags" for the filesystem.
    pub fn flags(&self) -> Flags {
        Flags::from_bits_truncate(self.flags)
    }
}

/// Reclaim constants
pub const _RECLAIM: u32 = 0b0001;
pub const _RECLAIMING: u32 = 0b0010;
pub const _CHECK_CLEAN: u32 = 0b0100;
pub const _CHECK_RECLAIM: u32 = 0b1000;

/// Rolled values.
pub const _PRE_FLAG: u32 = 0b00;
pub const _ALL_ROLLED: u32 = 0b01;
pub const _NEED_ROLL: u32 = 0b10;

/// Whether to optimize for space or time.
pub const _OPTTIME: u32 = 0b00;
pub const _OPTSPACE: u32 = 0b01;

pub const _CG_MAGIC: u32 = 0x090255;

/// A Cylinder Group
#[allow(dead_code)]
#[repr(C)]
#[derive(Debug)]
pub struct CylGroup {
    link: u32,              // Not used.
    magic: u32,             // Eric's birthday
    mtime: u32,             // Last modification time (oh dear)
    cgx: u32,               // Index of this cylinder group.
    ncyl: i16,              // Number of cylinders in this group
    niblk: i16,             // Number of inode blocks in this group
    ndblk: u32,             // Number of data blocks in this group
    cs: CylGroupSummary,    // Cylinder summary information
    rotor: u32,             // Position of last used block
    frotor: u32,            // Position of last used fragment
    irotor: u32,            // Position of last used inode
    frsum: [u32; MAX_FRAG], // Counts of available fragments
    btotoff: u32,           // block totals per cylinder
    boff: u32,              // Free block positions
    iusedoff: u32,          // Used inode map
    freeoff: u32,           // Free block map
    nextfreeoff: u32,       // Next available space
    _resv: [u32; 16],       // Reserved
}

/// The Root Inode Number
///
/// Inode numbers are origin 1; 0 is the "unused" indicator.
/// The first inode (1) is used to hold bad blocks.  Thus, the
/// root is at 2.
pub const ROOT_INODE: u32 = 2;

/// Number of direct block addresses in the inode.
const NDADDR: usize = 12;

/// Number of indirect block addresses in the inode.
/// Note that each entry in the inode array represents another
/// level of indirection, so that the first is singly-indirect,
/// the second is doubly-indirect, and the third is triply-indirect.
const NIADDR: usize = 3; // Number of indirect block address in inode

/// Fast Symbolic Link size
///
/// Fast symbolic links are an optimization where, if the filename the
/// link points to is short enough, the target path name is stored
/// directly in the inode itself.
const _FSL_SIZE: usize = (NDADDR + NIADDR - 1) * core::mem::size_of::<u32>();

/// The storage-resident version of an inode.
#[repr(C, align(128))]
#[derive(Debug)]
pub struct DInode {
    smode: u16,             // 0: mode and type of file
    nlink: u16,             // 2: number of links to file
    suid: u16,              // 4: owner's user id
    sgid: u16,              // 6: owner's group id
    lsize: u64,             // 8: number of bytes in file
    atime: u32,             // 16: time last accessed
    _atimes: u32,           // 20: atime spare
    mtime: u32,             // 24: time last modified
    _mtimes: u32,           // 28: mtime spare
    ctime: u32,             // 32: last time inode changed
    _ctimes: u32,           // 36: ctime spare
    dblocks: [u32; NDADDR], // 40: disk block addresses
    iblocks: [u32; NIADDR], // 88: indirect blocks
    flags: u32,             // 100: "cflags"
    blocks: u32,            // 104: Number 512 byte blocks actually used
    gen: u32,               // 108: generation number
    shadow: u32,            // 112: shadow inode
    uid: u32,               // 116: long EFT version of uid
    gid: u32,               // 120: long EFT version of gid
    oeftflag: u32,          // 124: extended attr directory ino, 0 = none
}

#[derive(Debug)]
pub struct FileSystem<'a> {
    sd: &'a [u8],
    pub sb: SuperBlock,
}

impl<'a> FileSystem<'a> {
    pub fn new(sd: &'a [u8]) -> FileSystem<'a> {
        let sb = SuperBlock::read(sd);
        FileSystem { sd, sb }
    }

    pub fn superblock(&self) -> &SuperBlock {
        &self.sb
    }

    pub fn root_inode(&self) -> Inode {
        Inode::new(self, ROOT_INODE).expect("root inode exists")
    }

    pub fn inode(&self, ino: u32) -> Result<Inode, ()> {
        Inode::new(self, ino)
    }

    pub fn fragsize(&self) -> usize {
        self.sb.fsize as usize
    }

    /// Returns the disk block number of a fragment.
    pub fn frags_to_sdblock(&self, fbno: usize) -> usize {
        self.sb.fsbtodb(fbno)
    }

    /// Returns the logical file block number for the given byte
    /// offset.
    pub fn logical_blockno(&self, offset: u64) -> usize {
        self.sb.lblkno(offset) as usize
    }

    /// Returns the number of inodes per fragment.
    #[allow(dead_code)]
    pub fn inodes_per_frag(&self) -> usize {
        self.sb.inopf() as usize
    }

    /// Returns the number of cylinder groups in the filesystem,
    /// as a Range, starting at zero.
    #[allow(dead_code)]
    pub fn cylgroups(&self) -> Range<u32> {
        0..self.sb.ncg
    }

    /// Returns the byte offset of the start of the data block
    /// region for the given cylinder group.
    #[allow(dead_code)]
    pub fn cylgroup_data_offset(&self, cylgrp: u32) -> usize {
        self.sb.cgdmin(cylgrp) as usize * self.fragsize()
    }

    /// Returns the number of indirect blocks spanned by a file
    /// system block.
    pub fn indir_span_per_block(&self) -> usize {
        self.sb.nindir as usize
    }

    /// Returns the logical fragment number in a block for a given
    /// file byte offset.
    pub fn logical_block_fragno(&self, offset: u64) -> usize {
        let offset = offset as usize;
        (offset % self.blocksize()) / self.fragsize()
    }

    /// Returns a the block size of the filesystem.
    pub fn blocksize(&self) -> usize {
        self.sb.bsize as usize
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Block<'a> {
    Hole,
    Sd(&'a [u8]),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum FileType {
    Unused = 0o00,
    Fifo = 0o01,
    Char = 0o02,
    Dir = 0o04,
    Block = 0o06,
    Regular = 0o10,
    SymLink = 0o12,
    ShadowInode = 0o13,
    Sock = 0o14,
    AttrDir = 0o16,
}

impl FileType {
    fn as_char(self) -> char {
        match self {
            FileType::Unused => 'X',
            FileType::Fifo => 'p',
            FileType::Char => 'c',
            FileType::Dir => 'd',
            FileType::Block => 'b',
            FileType::Regular => '-',
            FileType::SymLink => 'l',
            FileType::ShadowInode => 'I',
            FileType::Sock => 's',
            FileType::AttrDir => 'A',
        }
    }
}

bitstruct! {
    #[derive(Clone, Copy)]
    pub struct Mode(u16) {
        ox: bool = 0;
        ow: bool = 1;
        or: bool = 2;
        gx: bool = 3;
        gw: bool = 4;
        gr: bool = 5;
        ux: bool = 6;
        uw: bool = 7;
        ur: bool = 8;
        sticky: bool = 9;
        sgid: bool = 10;
        suid: bool = 11;
        typ: FileType = 12..=15;
    }
}

impl bitstruct::FromRaw<u8, FileType> for Mode {
    fn from_raw(raw: u8) -> FileType {
        match raw {
            0o01 => FileType::Fifo,
            0o02 => FileType::Char,
            0o04 => FileType::Dir,
            0o06 => FileType::Block,
            0o10 => FileType::Regular,
            0o12 => FileType::SymLink,
            0o13 => FileType::ShadowInode,
            0o14 => FileType::Sock,
            0o16 => FileType::AttrDir,
            _ => FileType::Unused,
        }
    }
}

impl bitstruct::IntoRaw<u8, FileType> for Mode {
    fn into_raw(bits: FileType) -> u8 {
        bits as u8
    }
}

impl fmt::Debug for Mode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fn alt(b: bool, t: char, f: char) -> char {
            if b {
                t
            } else {
                f
            }
        }
        f.write_char(self.typ().as_char())?;
        f.write_char(alt(self.ur(), 'r', '-'))?;
        f.write_char(alt(self.uw(), 'w', '-'))?;
        if !self.suid() {
            f.write_char(alt(self.ux(), 'x', '-'))?;
        } else {
            f.write_char(alt(self.ux(), 's', 'S'))?;
        }

        f.write_char(alt(self.gr(), 'r', '-'))?;
        f.write_char(alt(self.gw(), 'w', '-'))?;
        if !self.sgid() {
            f.write_char(alt(self.gx(), 'x', '-'))?;
        } else {
            f.write_char(alt(self.gx(), 's', 'S'))?;
        }

        f.write_char(alt(self.or(), 'r', '-'))?;
        f.write_char(alt(self.ow(), 'w', '-'))?;
        if !self.sticky() {
            f.write_char(alt(self.ox(), 'x', '-'))?;
        } else {
            f.write_char(alt(self.ox(), 't', 'T'))?;
        }
        Ok(())
    }
}

/// An in-memory representation of an inode, that associates the
/// inode with the underlying filesystem it came from.
#[derive(Debug)]
pub struct Inode<'a> {
    pub dinode: DInode,
    pub fs: &'a FileSystem<'a>,
}

impl<'a> Inode<'a> {
    /// Returns a new inode from the given filesystem.
    pub fn new(fs: &'a FileSystem<'a>, ino: u32) -> Result<Inode<'a>, ()> {
        let inoff = fs.sb.inode_offset(ino);
        let p = fs.sd.as_ptr().wrapping_add(inoff).cast::<DInode>();
        let dinode = unsafe { ptr::read_unaligned(p) };
        Ok(Inode { dinode, fs })
    }

    /// Returns the size of the file that this inode refers to.
    pub fn size(&self) -> usize {
        self.dinode.lsize as usize
    }

    /// Reads from an inode.
    pub fn read(&self, off: u64, buf: &mut [u8]) -> Result<usize, ()> {
        let mut off = off as usize;
        if off > MAX_OFFSET {
            return Err(());
        }
        if off > self.size() {
            return Ok(0);
        }
        let fragsize = self.fs.fragsize();
        let n = core::cmp::min(buf.len(), self.size() - off);
        let mut nread = 0;
        while nread < n {
            let frag_off: usize = off % fragsize;
            let m = cmp::min(n - nread, fragsize - frag_off);
            match self.bmap(off as u64)? {
                Block::Hole => {
                    buf[nread..nread + m].fill(0);
                }
                Block::Sd(bs) => {
                    buf[nread..nread + m].copy_from_slice(&bs[frag_off..frag_off + m]);
                }
            }
            off += m;
            nread += m;
        }
        Ok(n)
    }

    pub fn nlink(&self) -> u16 {
        self.dinode.nlink
    }

    pub fn uid(&self) -> u32 {
        self.dinode.uid
    }

    pub fn gid(&self) -> u32 {
        self.dinode.gid
    }

    /// Maps a byte offset in some file into a fragment-sized block
    /// from the the storage device.
    pub fn bmap(&self, off: u64) -> Result<Block, ()> {
        let fs = self.fs;
        let lbn = fs.logical_blockno(off) as usize;
        if lbn < NDADDR {
            let sdbn = self.dinode.dblocks[lbn] as usize;
            let offset = (sdbn + fs.logical_block_fragno(off)) * fs.fragsize();
            return Ok(Block::Sd(&fs.sd[offset..offset + fs.fragsize()]));
        }
        let mut lbn = lbn - NDADDR;
        let mut indir_span = 1;
        let mut indir_depth = 0;
        while indir_depth < NIADDR {
            indir_span *= fs.indir_span_per_block();
            if lbn < indir_span {
                break;
            }
            lbn -= indir_span;
            indir_depth += 1;
        }
        if indir_depth == NIADDR {
            // Too big.
            return Err(());
        }
        let mut nb = self.dinode.iblocks[indir_depth];
        for _ in 0..=indir_depth {
            let dblockno = fs.frags_to_sdblock(nb as usize);
            if dblockno == 0 {
                return Ok(Block::Hole);
            }
            indir_span /= fs.indir_span_per_block();
            let dboff = (lbn / indir_span) % fs.indir_span_per_block();
            let dbaddr = dblockno * DEV_BLOCK_SIZE + dboff * 4;
            let bs = &fs.sd[dbaddr..dbaddr + 4];
            nb = u32::from_ne_bytes([bs[0], bs[1], bs[2], bs[3]]);
            if nb == 0 {
                return Ok(Block::Hole);
            }
        }
        let sdbn = nb as usize;
        let offset = (sdbn + fs.logical_block_fragno(off)) * fs.fragsize();
        Ok(Block::Sd(&self.fs.sd[offset..offset + fs.fragsize()]))
    }

    pub fn mode(&self) -> Mode {
        Mode(self.dinode.smode)
    }
}

mod dir {
    use super::{FileType, Inode};
    use core::fmt;
    use core::mem;

    /// The maximum length of a name.
    pub const MAX_NAME_LEN: usize = 255;

    // Legnth of a diretory prefix (before the name).
    pub const PREFIX_LEN: usize = 8;

    pub struct Directory<'a> {
        pub(super) inode: Inode<'a>,
    }

    impl<'a> Directory<'a> {
        pub fn new(inode: Inode<'a>) -> Directory<'a> {
            let mode = inode.mode();
            assert_eq!(mode.typ(), FileType::Dir);
            Directory { inode }
        }

        pub fn iter(&self) -> Iter<'_> {
            Iter::new(&self)
        }
    }

    pub struct Iter<'a> {
        dir: &'a super::Directory<'a>,
        pos: u64,
    }

    impl<'a> Iter<'a> {
        pub fn new(dir: &'a Directory<'a>) -> Iter<'a> {
            let pos = 0;
            Iter { dir, pos }
        }
    }

    impl<'a> Iterator for Iter<'a> {
        type Item = Entry;

        fn next(&mut self) -> Option<Self::Item> {
            let mut buf = [0u8; PREFIX_LEN];
            let nread = self.dir.inode.read(self.pos, &mut buf).ok()?;
            if nread < PREFIX_LEN {
                return None;
            }
            let ino = u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]);
            let reclen = u16::from_ne_bytes([buf[4], buf[5]]) as usize;
            if reclen == 0 {
                return None;
            }
            let namelen = u16::from_ne_bytes([buf[6], buf[7]]) as usize;
            if reclen - PREFIX_LEN < namelen || namelen > MAX_NAME_LEN {
                return None;
            }
            let mut name = [0u8; MAX_NAME_LEN + 1];
            let dst = &mut name[..namelen];
            let namepos = self.pos + PREFIX_LEN as u64;
            let nread = self.dir.inode.read(namepos, dst).ok()?;
            if nread != namelen {
                return None;
            }
            let entry = Entry {
                ino,
                reclen: reclen as u16,
                namelen: namelen as u16,
                name,
            };
            self.pos += reclen as u64;
            Some(entry)
        }
    }

    /// The in-memory representation of a directory entry.
    #[repr(C)]
    pub struct Entry {
        ino: u32,
        reclen: u16,
        namelen: u16,
        name: [u8; MAX_NAME_LEN + 1],
    }

    impl Entry {
        pub fn dirsiz(&self) -> u16 {
            const BASE_SIZE: usize = mem::size_of::<Entry>() - MAX_NAME_LEN - 1; // c'mon dude; it's 264
            let name_size = (self.namelen + 1 + 3) & !3;
            BASE_SIZE as u16 + name_size
        }

        pub fn name(&self) -> &[u8] {
            let name = &self.name[..self.namelen as usize];
            if let Some(nul) = name.iter().position(|&b| b == 0u8) {
                &name[..nul]
            } else {
                name
            }
        }

        pub fn ino(&self) -> u32 {
            self.ino
        }
    }

    impl fmt::Debug for Entry {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
            writeln!(f, "Entry {{")?;
            writeln!(f, "    size: {}", self.dirsiz())?;
            writeln!(f, "    ino: {}", self.ino)?;
            writeln!(f, "    reclen: {}", self.reclen)?;
            writeln!(f, "    namelen: {}", self.namelen)?;
            let name = unsafe { core::str::from_utf8_unchecked(self.name()) };
            writeln!(f, "    name = {name}")?;
            write!(f, "}}")
        }
    }
}

pub use dir::Directory;
