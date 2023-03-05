#![allow(unused)]
use alloc::boxed::Box;
use alloc::sync::{Arc, Weak};
use fatfs::{DefaultTimeProvider, Dir, IoBase, LossyOemCpConverter, Read, Seek, SeekFrom, Write};
use rvfs::dentry::{DirEntry, DirFlags};
use rvfs::inode::{Inode, InodeMode, simple_statfs};
use rvfs::mount::MountFlags;
use rvfs::StrResult;
use rvfs::superblock::{DataOps, Device, FileSystemAttr, FileSystemType, SuperBlock, SuperBlockInner, SuperBlockOps};
use spin::Mutex;
use crate::{FatInode, FatInodeType};
use crate::file::{FAT_DENTRY_OPS, FAT_DIR_FILE_OPS};
use crate::inode::{FAT_INODE_DIR_OPS};


pub struct FatDevice{
    pos: usize,
    device_file: Arc<dyn Device>
}
impl FatDevice{
    pub fn new(device:Arc<dyn Device>)->Self{
        Self{
            pos:0,
            device_file:device
        }
    }
}

impl IoBase for FatDevice { type Error = (); }

impl Write for FatDevice{
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        todo!()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        todo!()
    }
}
impl Read for FatDevice{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        todo!()
    }
}


impl Seek for FatDevice{
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        todo!()
    }
}

const FATFS_SB_OPS: SuperBlockOps = {
    let mut sb_ops = SuperBlockOps::empty();
    sb_ops.stat_fs = simple_statfs;
    sb_ops.sync_fs = fat_sync_fs;
    sb_ops
};

const FAT: FileSystemType = {
    FileSystemType::new(
        "fat",
        FileSystemAttr::empty(),
        fat_get_super_blk,
        fat_kill_super_blk,
    )
};

fn fat_get_super_blk(fs_type: Arc<FileSystemType>, flags: MountFlags, dev_name: &str, data: Option<Box<dyn DataOps>>,) -> StrResult<Arc<SuperBlock>> {
    assert!(data.is_some());
    let device = data.as_ref().unwrap().as_ref().device(dev_name);
    assert!(device.is_some());
    let device = device.unwrap();
    let fat_device = FatDevice::new(device.clone());
    let fs = fatfs::FileSystem::new(fat_device, fatfs::FsOptions::new()).unwrap();
    let stats = fs.stats();
    if stats.is_err(){
        return Err("read fat data error")
    }
    let stats = stats.unwrap();
    let sb_blk = SuperBlock{
        dev_desc: fs.volume_id(),
        device:Some(device),
        block_size: stats.cluster_size(),
        dirty_flag: false,
        file_max_bytes: usize::MAX,
        mount_flag: flags,
        magic: 0,
        file_system_type: Default::default(),
        super_block_ops: FATFS_SB_OPS,
        blk_dev_name: fs.volume_label(),
        data,
        inner:Mutex::new(SuperBlockInner::empty())
    };
    // set the root dentry for super block
    let sb_blk = Arc::new(sb_blk);
    let inode = fat_root_inode(sb_blk.clone(),fs.root_dir());
    let dentry = DirEntry::new(DirFlags::empty(),inode,FAT_DENTRY_OPS,Weak::new(),"/");
    sb_blk.update_root(Arc::new(dentry));
    // inert the super block into file system type
    fs_type.insert_super_blk(sb_blk.clone());
    Ok(sb_blk)
}


fn fat_kill_super_blk(super_blk: Arc<SuperBlock>) {
    let ops = super_blk.super_block_ops.sync_fs;
    ops(super_blk);
}

fn fat_sync_fs(sb_blk: Arc<SuperBlock>) -> StrResult<()>{
    let device = sb_blk.device.as_ref().unwrap().clone();
    let fat_device = FatDevice::new(device);
    let fs = fatfs::FileSystem::new(fat_device, fatfs::FsOptions::new()).unwrap();
    let res = fs.unmount();
    if res.is_err(){
        return Err("sync error");
    }
    Ok(())
}

/// create the root inode for fat file system
fn fat_root_inode(sb_blk:Arc<SuperBlock>,dir:Dir<FatDevice,DefaultTimeProvider,LossyOemCpConverter>) -> Arc<Inode> {
    let device = sb_blk.device.as_ref().unwrap().clone();
    let inode = Inode::new(
        sb_blk,
        0,
        0,
        FAT_INODE_DIR_OPS,
        FAT_DIR_FILE_OPS,
        None,
        InodeMode::S_DIR
    );
    let parent = Arc::new(Mutex::new(dir));
    let fat_inode = FatInode::new(parent.clone(),FatInodeType::Dir(parent));
    inode.access_inner().data = Some(Box::new(fat_inode));
    Arc::new(inode)
}