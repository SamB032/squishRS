#[test]
fn test_roundtrip_pack_unpack() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let input_dir = temp.path().join("input");
    let output_dir = temp.path().join("output");
    let archive_path = temp.path().join("test.squish");

    std::fs::create_dir(&input_dir)?;
    std::fs::write(input_dir.join("file.txt"), b"hello squish")?;

    // Pack
    let files = squishrs::fsutil::directory::walk_dir(&input_dir)?;
    let mut writer = squishrs::archive::ArchiveWriter::new(&input_dir, &archive_path, None)?;
    writer.pack(&files)?;

    // Unpack
    let mut reader = squishrs::archive::ArchiveReader::new(&archive_path)?;
    reader.unpack(&output_dir, None)?;

    // Compare files
    let orig = std::fs::read(input_dir.join("file.txt"))?;
    let extracted = std::fs::read(output_dir.join("file.txt"))?;
    assert_eq!(orig, extracted);

    Ok(())
}
