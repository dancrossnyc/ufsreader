// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
    println!("root inode: {:#x?}", root_inode);
    let rootdir = ufs::Directory::new(&root_inode);
    dump_dir(&fs, &rootdir);

    let kernel_inode = fs.namei(b"/kernel").expect("/kernel exists");
    println!("kernel mode: {:?}", kernel_inode.mode());
    let kerneldir = ufs::Directory::new(&kernel_inode);
    dump_dir(&fs, &kerneldir);

    let amd64_inode = fs.namei(b"/kernel/amd64").expect("/kernel/amd64 exists");
    println!("/kernel/amd64 mode: {:?}", amd64_inode.mode());
    let amd64dir = ufs::Directory::new(&amd64_inode);
    dump_dir(&fs, &amd64dir);

    let genunix_inode = fs
        .namei(b"/kernel/amd64/genunix")
        .expect("/kernel/amd64/genunix exists");
    println!("genunix inode: {:#x?}", genunix_inode);
    let mut genunixfile = vec![0u8; genunix_inode.size()];
    genunix_inode
        .read(0, &mut genunixfile)
        .expect("read /kernel/amd64/genunix");
    dump_file("target/tmp.genunix", &genunixfile);

    let etc_inode = fs.namei(b"/etc").expect("/etc exists");
    println!("etc mode: {:?}", etc_inode.mode());
    let etcdir = ufs::Directory::new(&etc_inode);
    dump_dir(&fs, &etcdir);

    let driver_aliases_inode = fs
        .namei(b"/etc/driver_aliases")
        .expect("/etc/driver_aliases exists");
    println!("driver_aliases mode: {:?}", driver_aliases_inode.mode());
    let mut driver_aliases_file = vec![0u8; driver_aliases_inode.size()];
    driver_aliases_inode
        .read(0, &mut driver_aliases_file)
        .expect("read /etc/driver_aliases");
    let da = unsafe { core::str::from_utf8_unchecked(&driver_aliases_file) };
    println!("Driver aliases content: |{da}|");

    let platform_inode = fs.namei(b"/platform").expect("/platform exists");
    println!("platform mode: {:?}", platform_inode.mode());
    let platformdir = ufs::Directory::new(&platform_inode);
    dump_dir(&fs, &platformdir);

    let oxide_inode = fs
        .namei(b"/platform/oxide")
        .expect("/platform/oxide exists");
    println!("oxide mode: {:?}", oxide_inode.mode());
    let oxidedir = ufs::Directory::new(&oxide_inode);
    dump_dir(&fs, &oxidedir);

    let kernel_inode = fs
        .namei(b"/platform/oxide/kernel")
        .expect("/platform/oxide/kernel exists");
    println!("kernel mode: {:?}", kernel_inode.mode());
    let kerneldir = ufs::Directory::new(&kernel_inode);
    dump_dir(&fs, &kerneldir);

    let amd64_inode = fs
        .namei(b"/platform/oxide/kernel/amd64")
        .expect("/platform/oxide/kernel/amd64 exists");
    println!("amd64 mode: {:?}", amd64_inode.mode());
    let amd64dir = ufs::Directory::new(&amd64_inode);
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

    let log_inode = fs.namei(b"/etc/TIMEZONE").expect("/etc/TIMEZONE exists");
    println!("log: {log_inode:#x?}");
}

fn dump_dir(fs: &ufs::FileSystem<'_>, dir: &ufs::Directory<'_>) {
    // for dentry in dir.iter() {
    //     println!("dir: {dentry:#?}");
    // }
    for dentry in dir.iter() {
        let file = fs.inode(dentry.ino()).expect("got file");
        println!(
            "#{:<4} {:?} {:<2} {:<3} {:<3} {:>8} {}",
            file.ino(),
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
