use std::env;
use std::fs;
use std::ptr;

mod ufs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: ufsreader fs");
        panic!("fs")
    }
    let disk = fs::read(&args[1]).expect("read filesystem");
    let fs = ufs::FileSystem::new(&disk);
    let root_inode = fs.root_inode();
    println!(
        "root_inode at {offset:?}",
        offset = root_inode.bmap(0).map(|v| v * fs.sb.fragsize() as u32)
    );
    let _ = root_inode.bmap(1000000);
    let dbytes = root_inode.dinode.dbaddr(&fs.sb, 0).unwrap() as usize;
    println!("Contents at {dbytes}");
    dump_root(&disk[dbytes..dbytes + root_inode.dinode.len()]);
}

fn dump_root(mut dir: &[u8]) {
    loop {
        let Some((d, ndir)) = ufs::Dirent::first(dir) else {
            break;
        };
        dir = ndir;
        println!("dir: {d:#?}");
    }
}
