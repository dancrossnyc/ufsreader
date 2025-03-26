use std::env;
use std::fs;

mod ufs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: ufsreader fs");
        panic!("fs")
    }
    let disk = fs::read(&args[1]).expect("read filesystem");
    let fs = ufs::FileSystem::new(&disk);
    println!("fs.state = {:?}", fs.superblock().state());
    println!("fs.flags = {:?}", fs.superblock().flags());
    let root_inode = fs.root_inode();
    println!("root mode: {:?}", root_inode.mode());
    println!(
        "root_inode directory contents at {offset:?}",
        offset = match root_inode.bmap(0).unwrap() {
            ufs::Block::Sd(v) => v.as_ptr().addr() - disk.as_ptr().addr(),
            ufs::Block::Hole => 0,
        }
    );
    let rootdir = ufs::Directory::new(root_inode);
    dump_dir(&fs, &rootdir);

    let kernel_inode = fs.inode(44).expect("/kernel exists");
    println!("kernel mode: {:?}", kernel_inode.mode());
    let kerneldir = ufs::Directory::new(kernel_inode);
    dump_dir(&fs, &kerneldir);

    let amd64_inode = fs.inode(45).expect("/kernel/amd64 exists");
    println!("/kernel/amd64 mode: {:?}", amd64_inode.mode());
    let amd64dir = ufs::Directory::new(amd64_inode);
    dump_dir(&fs, &amd64dir);

    let genunix_inode = fs.inode(239).expect("/kernel/amd64/genunix exists");
    println!("genunix inode: {:#x?}", genunix_inode);
    let mut genunixfile = vec![0u8; genunix_inode.size()];
    genunix_inode
        .read(0, &mut genunixfile)
        .expect("read /kernel/amd64/genunix");
    dump_file("target/tmp.genunix", &genunixfile);

    let etc_inode = fs.inode(13).expect("/etc exists");
    println!("etc mode: {:?}", etc_inode.mode());
    let etcdir = ufs::Directory::new(etc_inode);
    dump_dir(&fs, &etcdir);

    let driver_aliases_inode = fs.inode(174).expect("/etc/driver.aliases exists");
    println!("driver_aliases mode: {:?}", driver_aliases_inode.mode());
    let mut driver_aliases_file = vec![0u8; driver_aliases_inode.size()];
    driver_aliases_inode
        .read(0, &mut driver_aliases_file)
        .expect("read /etc/driver.aliases");
    let da = unsafe { core::str::from_utf8_unchecked(&driver_aliases_file) };
    println!("{da}");

    let platform_inode = fs.inode(97).expect("/platform exists");
    println!("platform mode: {:?}", platform_inode.mode());
    let platformdir = ufs::Directory::new(platform_inode);
    dump_dir(&fs, &platformdir);

    let oxide_inode = fs.inode(98).expect("/platform/oxide exists");
    println!("oxide mode: {:?}", oxide_inode.mode());
    let oxidedir = ufs::Directory::new(oxide_inode);
    dump_dir(&fs, &oxidedir);

    let kernel_inode = fs.inode(99).expect("/platform/oxide/kernel exists");
    println!("kernel mode: {:?}", kernel_inode.mode());
    let kerneldir = ufs::Directory::new(kernel_inode);
    dump_dir(&fs, &kerneldir);

    let amd64_inode = fs.inode(100).expect("/platform/oxide/kernel/amd64 exists");
    println!("amd64 mode: {:?}", amd64_inode.mode());
    let amd64dir = ufs::Directory::new(amd64_inode);
    dump_dir(&fs, &amd64dir);

    let unix_inode = fs
        .namei(b"/platform/oxide/kernel/amd64/unix")
        .expect("/platform/oxide/kernel/amd64/unix exists");
    println!("unix: {:#x?}", unix_inode);
    let mut unixfile = vec![0u8; unix_inode.size()];
    unix_inode
        .read(0, &mut unixfile)
        .expect("read /platform/oxide/kernel/amd64/unix");
    dump_file("target/tmp.unix", &unixfile);
}

fn dump_dir(fs: &ufs::FileSystem<'_>, dir: &ufs::Directory<'_>) {
    // for dentry in dir.iter() {
    //     println!("dir: {dentry:#?}");
    // }
    for dentry in dir.iter() {
        let file = fs.inode(dentry.ino()).expect("got file");
        println!(
            "{:?} {:<2} {:<3} {:<3} {:>8} {}",
            file.mode(),
            file.nlink(),
            file.uid(),
            file.gid(),
            file.size(),
            unsafe { core::str::from_utf8_unchecked(dentry.name()) }
        );
    }
}

fn dump_file(name: &str, file: &[u8]) {
    use std::fs::File;
    use std::io::prelude::*;
    let mut f = File::create(name).expect("created {name}");
    f.write_all(file).expect("wrote {name}");
    println!("dumped '{name}' (size: {size})", size = file.len());
}
