mod lib;
mod print_message;

use chrono::{DateTime, FixedOffset, Utc};
use clap::*;
use csv::Writer;
use serde_json::*;
use std::fs;
use std::process;

fn parse(path: &str) -> Value {
  let data = fs::read_to_string(path).unwrap();
  serde_json::from_str(&data).unwrap()
}

fn json_to_sizai_vec(
  j_map: &serde_json::Map<std::string::String, serde_json::Value>,
) -> Vec<lib::Sizai> {
  let keys = j_map.keys();
  let mut s_vec = vec![];
  for key in keys {
    let v = j_map.get(key).unwrap().as_str().unwrap().to_owned();
    s_vec.push(lib::make_sizai(key.to_owned(), v))
  }
  s_vec
}

fn json_to_sandan_vec(
  j_map: &serde_json::Map<std::string::String, serde_json::Value>,
) -> Vec<lib::Sandan> {
  let keys = j_map.keys();
  let mut s_vec = vec![];
  for key in keys {
    let v = j_map.get(key).unwrap().as_str().unwrap().to_owned();
    s_vec.push(lib::make_sandan(key.to_owned(), v))
  }
  s_vec
}

fn csv_to_lend_data(csv_record: &csv::StringRecord) -> lib::LendData {
  // "操作時刻", "どの種類の操作か", "品名", "貸出先", "削除・編集する先の操作番号", "編集後の品名", "編集後の貸出先", "操作番号",
  let time = DateTime::parse_from_rfc3339(csv_record.get(0).unwrap()).unwrap();
  let num = csv_record.get(7).unwrap().parse().unwrap();
  let lend_type_str: &str = &csv_record.get(1).unwrap().to_owned().to_ascii_lowercase();
  let lend_type = match lend_type_str {
    "lend" => {
      // 貸出：「貸し出した品名」と「貸出先」
      let product = csv_record.get(2).unwrap().to_owned();
      let destination_opt = match csv_record.get(3).map(|s| s.to_owned()) {
        None => None,
        Some(s) => {
          if s.is_empty() {
            None
          } else {
            Some(s)
          }
        }
      };
      lib::LendType::Lend(product, destination_opt)
    }
    "return" => {
      // 返却：「返された品名」と「返却先」
      let product = csv_record.get(2).unwrap().to_owned();
      let destination_opt = match csv_record.get(3).map(|s| s.to_owned()) {
        None => None,
        Some(s) => {
          if s.is_empty() {
            None
          } else {
            Some(s)
          }
        }
      };
      lib::LendType::Return(product, destination_opt)
    }
    "edit" => {
      // 編集：「編集する操作対象に付けられた通し番号」と「編集後の品名」と「編集後の貸出先」
      let num = csv_record.get(4).unwrap().parse().unwrap();
      let new_product = csv_record.get(5).unwrap().to_owned();
      let new_destination_opt = match csv_record.get(3).map(|s| s.to_owned()) {
        None => None,
        Some(s) => {
          if s.is_empty() {
            None
          } else {
            Some(s)
          }
        }
      };
      lib::LendType::Edit(num, new_product, new_destination_opt)
    }
    "remove" => {
      // 編集：「削除する操作対象に付けられた通し番号」
      let num = csv_record.get(4).unwrap().parse().unwrap();
      lib::LendType::Remove(num)
    }
    _ => panic!(),
  };
  lib::LendData {
    time,
    lend_type,
    num,
  }
}

fn csv_data_to_lend_data(csv_data: Vec<csv::StringRecord>) -> Vec<lib::LendData> {
  csv_data.iter().map(|r| csv_to_lend_data(r)).collect()
}

fn csv_file_name_to_lend_data(file_name: String) -> Vec<lib::LendData> {
  let csv_reader_r = csv::Reader::from_path(file_name);
  match csv_reader_r {
    Err(_) => Vec::new(),
    Ok(csv_reader) => {
      let mut csv_reader = csv_reader;
      // for文を使ってリスト化
      let mut csv_data_vec = vec![];
      for csv_data in csv_reader.records() {
        csv_data_vec.push(csv_data.unwrap())
      }
      csv_data_to_lend_data(csv_data_vec)
    }
  }
}

fn lend_data_lst_to_output(path: &str, csv_data_lst: Vec<lib::LendData>) {
  let empty_str = "";
  let mut wtr = Writer::from_path(path).unwrap();
  wtr
    .write_record(&[
      "操作時刻",
      "どの種類の操作か",
      "品名",
      "貸出先",
      "削除・編集する先の操作番号",
      "編集後の品名",
      "編集後の貸出先",
      "操作番号",
    ])
    .unwrap();
  for lend_data in csv_data_lst.iter() {
    let time = lend_data.clone().time;
    let time_str = &time.to_rfc3339();
    let num_str = &lend_data.num.to_string();
    let lend_type = lend_data.clone().lend_type;
    match lend_type {
      // 貸出：「貸し出した品名」と「貸出先」
      lib::LendType::Lend(product_name, destination_opt) => {
        let destination_string = match destination_opt {
          None => String::new(),
          Some(s) => s,
        };
        wtr
          .write_record(&[
            time_str,
            "Lend",
            &product_name,
            &destination_string,
            empty_str,
            empty_str,
            empty_str,
            num_str,
          ])
          .unwrap();
      }
      // 返却：「返された品名」と「返却先」
      lib::LendType::Return(product_name, destination_opt) => {
        let destination_string = match destination_opt {
          None => String::new(),
          Some(s) => s,
        };
        wtr
          .write_record(&[
            time_str,
            "Return",
            &product_name,
            &destination_string,
            empty_str,
            empty_str,
            empty_str,
            num_str,
          ])
          .unwrap();
      }
      // 編集：「編集する操作対象に付けられた通し番号」と「編集後の品名」と「編集後の貸出先」
      lib::LendType::Edit(num, new_product_name, new_destination_opt) => {
        let new_destination_string = match new_destination_opt {
          None => String::new(),
          Some(s) => s,
        };
        wtr
          .write_record(&[
            time_str,
            "Edit",
            empty_str,
            empty_str,
            &num.to_string(),
            &new_product_name,
            &new_destination_string,
            num_str,
          ])
          .unwrap();
      }
      // 削除：「削除する操作対象に付けられた通し番号」
      lib::LendType::Remove(num) => {
        wtr
          .write_record(&[
            time_str,
            "Remove",
            empty_str,
            empty_str,
            &num.to_string(),
            empty_str,
            empty_str,
            num_str,
          ])
          .unwrap();
      }
    }
  }
  wtr.flush().unwrap()
}

fn check_lend_product_num(data: &lib::LendData, product_num: &str) -> bool {
  match data.clone().lend_type {
    lib::LendType::Lend(data_product_num, _) => data_product_num == *product_num,
    _ => false,
  }
}

#[allow(unused_assignments)]
fn main() {
  // 引数の処理
  let matches = App::new("dlm")
    .version("0.1.0")
    .author("(C) 2020 149th文化祭準備員会（2021年1月開催）電気係SC")
    .about("電気係の使用する貸出管理ソフトウェア")
    .arg(
      Arg::with_name("data_file_name")
        .value_name("FILE")
        .help("貸出先や貸出時刻などのデータを記録したファイル（CSV形式）")
        .takes_value(true)
        .required(true),
    )
    .arg(
      Arg::with_name("config_file_name")
        .short("c")
        .long("config")
        .value_name("FILE")
        .help("資材や参団の名前の対応ファイル（JSON形式）")
        .takes_value(true),
    )
    .get_matches();

  let data_file_name_opt = matches.value_of("data_file_name");
  let data_file_name = match data_file_name_opt {
    Some(s) => s,
    None => {
      eprintln!("ファイル名を入力してください！");
      process::exit(0)
    }
  };
  let config_file_name_opt = matches.value_of("config_file_name");
  let config_data: lib::ConfigData = match config_file_name_opt {
    None => lib::make_config_data(Vec::new(), Vec::new()),
    Some(config_file_name) => {
      let json_data = parse(config_file_name);
      let sizai_json_data = json_data["sizai"].as_object().unwrap();
      let sizai_data_vec = json_to_sizai_vec(sizai_json_data);
      let sandan_json_data = json_data["sandan"].as_object().unwrap();
      let sandan_data_vec = json_to_sandan_vec(sandan_json_data);
      lib::make_config_data(sizai_data_vec, sandan_data_vec)
    }
  };
  let mut arg_command_history_vec: Vec<String> = Vec::new();
  print_message::print_start();
  loop {
    print_message::print_restart();
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).ok();
    s = s.trim().to_owned();
    arg_command_history_vec.push(s.clone());
    let arg_str_vec: Vec<&str> = s.split_whitespace().collect();
    let arg = lib::parse_arg(arg_str_vec);
    match arg {
      lib::Arg::Null => (),
      lib::Arg::Exit => break,
      lib::Arg::Help => print_message::print_help(),
      lib::Arg::NotFoundCommandName(name) => print_message::print_not_found_command_name(name),
      lib::Arg::MissingArgument => print_message::print_missing_argument(),
      lib::Arg::History(n) => print_message::print_history(&arg_command_history_vec, n),
      lib::Arg::Show => {
        let lend_data = csv_file_name_to_lend_data(data_file_name.to_owned());
        let lend_data_str = lib::make_lend_data_str(lend_data, config_data.clone());
        println!("操作番号   時刻               貸出品                                 貸出先");
        println!("{}", lend_data_str)
      }
      lib::Arg::Check => {
        println!("検査を開始します\n--- --- ---\n");
        let lend_data_lst =
          lib::organize_lend_data(&csv_file_name_to_lend_data(data_file_name.to_owned()));
        for lend_data in lend_data_lst.iter() {
          let lend_type = &lend_data.lend_type;
          let mut lend_stack: Vec<&lib::LendData> = Vec::new();
          match lend_type {
            lib::LendType::Lend(product_num, _) => {
              match lend_stack
                .iter()
                .find(|data| check_lend_product_num(data, product_num))
              {
                None => (),
                Some(_) => println!("- {}が2重に貸し出されています\n", product_num),
              }
              lend_stack.push(lend_data)
            }
            lib::LendType::Return(product_num, _) => {
              match lend_stack
                .iter()
                .find(|data| check_lend_product_num(data, product_num))
              {
                None => println!(
                  "- {}が貸し出されていないにもかかわらず返却されたことになっています\n",
                  product_num
                ),
                Some(_) => (),
              }
              let new_lend_stack = lend_stack
                .iter()
                .filter(|data| !(check_lend_product_num(data, product_num)))
                .cloned()
                .collect();
              lend_stack = new_lend_stack;
            }
            // これ以外は無いはずなので考慮しない
            _ => {}
          }
        }
        println!("--- --- ---\n検査を終了しました\n");
      }
      lib::Arg::Lend(product_num, destination_num_opt) => {
        let mut lend_data = csv_file_name_to_lend_data(data_file_name.to_owned());
        let lend_data_len = lend_data.len();
        let time_fixed_offset = Utc::now().with_timezone(&FixedOffset::east(9 * 3600));
        // 検査を行う
        // 検査を通過したらリストに登録してファイル更新
        match lend_data
          .iter()
          .find(|data| check_lend_product_num(data, &product_num))
        {
          None => {
            lend_data.push(lib::LendData {
              time: time_fixed_offset,
              lend_type: lib::LendType::Lend(product_num.clone(), destination_num_opt.clone()),
              num: (lend_data_len as isize + 1),
            });
            lend_data_lst_to_output(data_file_name, lend_data);
            print_message::print_lend_success(&product_num, &destination_num_opt);
          }
          Some(_) => println!(
            "!  {}が既に貸し出されているのでこの操作を行うことはできません\n",
            product_num
          ),
        }
      }
      lib::Arg::Return(product_num, destination_num_opt) => {
        let mut lend_data = csv_file_name_to_lend_data(data_file_name.to_owned());
        let lend_data_len = lend_data.len();
        let time_fixed_offset = Utc::now().with_timezone(&FixedOffset::east(9 * 3600));
        // 検査を行う
        // 検査を通過したらリストに登録してファイル更新
        match lend_data
          .iter()
          .find(|data| check_lend_product_num(data, &product_num))
        {
          None => println!(
            "!  {}がまだ貸し出されてないのでこの操作を行うことは出来ません\n",
            product_num
          ),
          Some(_) => {
            lend_data.push(lib::LendData {
              time: time_fixed_offset,
              lend_type: lib::LendType::Return(product_num.clone(), destination_num_opt.clone()),
              num: (lend_data_len as isize + 1),
            });
            lend_data_lst_to_output(data_file_name, lend_data);
            print_message::print_return_success(&product_num, &destination_num_opt);
          }
        }
      }
      lib::Arg::Edit(num, new_product_num, new_destination_num_opt) => {
        let mut lend_data = csv_file_name_to_lend_data(data_file_name.to_owned());
        let lend_data_len = lend_data.len();
        let time_fixed_offset = Utc::now().with_timezone(&FixedOffset::east(9 * 3600));
        let data = lib::get_lend_data(&lend_data, num).unwrap();
        let data_str = data.to_string();
        println!(
          "{}\nという{}番の操作の品名を\"{}\"に{}変更します\n本当に良いですか？[Y/n]\n    >",
          data_str,
          num,
          new_product_num,
          (match new_destination_num_opt.clone() {
            None => String::new(),
            Some(s) => format!("、相手を\"{}\"に", s),
          })
        );
        let mut s = String::new();
        std::io::stdin().read_line(&mut s).ok();
        let s: &str = &s.trim().to_owned().to_ascii_lowercase();
        match s {
          "n" => println!("操作を中止しました"),
          _ => {
            lend_data.push(lib::LendData {
              time: time_fixed_offset,
              lend_type: lib::LendType::Edit(num, new_product_num, new_destination_num_opt),
              num: (lend_data_len as isize + 1),
            });
            lend_data_lst_to_output(data_file_name, lend_data);
          }
        }
      }
      lib::Arg::Remove(num) => {
        let mut lend_data = csv_file_name_to_lend_data(data_file_name.to_owned());
        let lend_data_len = lend_data.len();
        let time_fixed_offset = Utc::now().with_timezone(&FixedOffset::east(9 * 3600));
        let data = lib::get_lend_data(&lend_data, num).unwrap();
        let data_str = data.to_string();
        println!(
          "{}\nという{}番の操作を無かったことにします\n本当に良いですか？[Y/n]\n    >",
          data_str, num
        );
        let mut s = String::new();
        std::io::stdin().read_line(&mut s).ok();
        let s: &str = &s.trim().to_owned().to_ascii_lowercase();
        match s {
          "n" => println!("操作を中止しました"),
          _ => {
            lend_data.push(lib::LendData {
              time: time_fixed_offset,
              lend_type: lib::LendType::Remove(num),
              num: (lend_data_len as isize + 1),
            });
            lend_data_lst_to_output(data_file_name, lend_data);
          }
        }
      }
      lib::Arg::AllPrint => {
        let lend_data_lst = csv_file_name_to_lend_data(data_file_name.to_owned());
        for lend_data in lend_data_lst {
          println!("{}", lend_data.to_string())
        }
      }
    };
  }
}
