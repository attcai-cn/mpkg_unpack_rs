use std::fs::{self, File};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::collections::VecDeque;

const BUFFER_SIZE: usize = 1024 * 1024; // 1MB

/// 读取4字节的整数 (小端序)
fn read_int32<R: Read>(reader: &mut R) -> io::Result<u32> {
    let mut buffer = [0u8; 4];
    reader.read_exact(&mut buffer)?;
    Ok(u32::from_le_bytes(buffer))
}

/// 复制流中的数据到目标流
fn copy_stream_data<R: Read, W: Write>(input: &mut R, output: &mut W, length: u64) -> io::Result<()> {
    let mut remaining = length;
    let mut buffer = vec![0u8; BUFFER_SIZE];

    while remaining > 0 {
        let to_read = std::cmp::min(buffer.len() as u64, remaining) as usize;
        let bytes_read = input.read(&mut buffer[..to_read])?;
        if bytes_read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "文件提前终止，预期长度：{} 剩余：{}",
                    length, remaining
                ),
            ));
        }
        output.write_all(&buffer[..bytes_read])?;
        remaining -= bytes_read as u64;
    }
    Ok(())
}

/// 解包单个MPKG文件
fn unpack_mpkg<P: AsRef<Path>>(input_file: P, output_dir: P) -> io::Result<()> {
    let input_file = input_file.as_ref();
    let output_dir = output_dir.as_ref();

    let mut input_stream = BufReader::new(File::open(input_file)?);

    // 创建输出文件夹，以MPKG文件名为文件夹名
    let unpacked_folder = output_dir.join(input_file.file_stem().unwrap_or_default());
    fs::create_dir_all(&unpacked_folder)?;

    // 读取头部信息
    let header_length = read_int32(&mut input_stream)?;
    let mut header_bytes = vec![0u8; header_length as usize];
    input_stream.read_exact(&mut header_bytes)?;
    let header_str = String::from_utf8_lossy(&header_bytes);
    println!("文件格式版本：{}", header_str);

    // 读取文件数量
    let file_count = read_int32(&mut input_stream)?;
    println!("发现文件数量：{}", file_count);

    // 构建文件列表
    let mut file_list = Vec::with_capacity(file_count as usize);
    for _ in 0..file_count {
        let name_length = read_int32(&mut input_stream)?;
        let mut name_bytes = vec![0u8; name_length as usize];
        input_stream.read_exact(&mut name_bytes)?;
        let file_name = String::from_utf8_lossy(&name_bytes).to_string();

        // 跳过未知字段 (4字节)
        input_stream.seek(SeekFrom::Current(4))?;

        // 读取文件大小
        let file_size = read_int32(&mut input_stream)?;
        file_list.push((file_name, file_size));
    }

    // 逐个解包文件到指定文件夹
    for (i, (file_name, file_size)) in file_list.iter().enumerate() {
        println!(
            "正在解包文件 {}/{} : {}",
            i + 1,
            file_list.len(),
            file_name
        );

        // 创建文件夹，确保路径存在
        let full_output_path = unpacked_folder.join(&file_name);
        if let Some(parent_dir) = full_output_path.parent() {
            fs::create_dir_all(parent_dir)?;
        }

        // 打开输出文件
        let mut output_stream = File::create(&full_output_path)?;

        // 复制数据
        copy_stream_data(&mut input_stream, &mut output_stream, *file_size as u64)?;
        println!("文件解包完成: {}", file_name);
    }

    println!("解包成功完成！");
    Ok(())
}

fn main() -> io::Result<()> {
    use std::io::Write;

    print!("请输入包含MPKG文件的文件夹路径：");
    io::stdout().flush()?; // 确保提示符被显示
    let mut input_folder = String::new();
    io::stdin().read_line(&mut input_folder)?;
    let input_folder = input_folder.trim();

    print!("请输入解包输出文件夹路径：");
    io::stdout().flush()?; 
    let mut output_folder = String::new();
    io::stdin().read_line(&mut output_folder)?;
    let output_folder = output_folder.trim();

    let input_folder_path = Path::new(input_folder);

    // 检查文件夹是否存在
    if !input_folder_path.is_dir() {
        eprintln!("无效的文件夹路径！");
        std::process::exit(1);
    }

    // 指定输出路径
    let output_dir = Path::new(output_folder);

    // 遍历文件夹中的所有MPKG文件并解包
    for entry in fs::read_dir(input_folder_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "mpkg") {
            println!("正在处理文件: {}", path.file_name().unwrap_or_default().to_string_lossy());
            match unpack_mpkg(&path, output_dir) {
                Ok(()) => println!("成功解包: {}", path.display()),
                Err(e) => eprintln!("解包失败: {}: {}", path.display(), e),
            }
        }
    }

    Ok(())
}