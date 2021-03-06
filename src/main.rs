mod lib;
mod print_message;

use chrono::{DateTime, FixedOffset, Utc};
use clap::*;
use csv::Writer;
use serde_json::*;
use std::fs;
use std::process;

// パスからファイルの中身を読み取ってserde_jsonで提供される関数でデータ化する
fn path_to_json_data(path: &str) -> Option<Value> {
  let data = fs::read_to_string(path).ok()?;
  serde_json::from_str(&data).ok()?
}

// 一行のCSVデータから一つの貸出返却関係のデータを作る
fn csv_to_lend_data(csv_record: &csv::StringRecord) -> lib::LendData {
  // "操作時刻", "どの種類の操作か", "品名", "貸出先", "削除・編集する先の操作番号", "編集後の品名", "編集後の貸出先", "操作番号",
  // 時刻と操作番号は全ての操作に置いて必要なので先に取得してそれぞれのデータに直す
  let time: DateTime<FixedOffset> =
    DateTime::parse_from_rfc3339(csv_record.get(0).unwrap()).unwrap();
  let num: isize = csv_record.get(7).unwrap().parse().unwrap();
  let lend_type_str: &str = &csv_record.get(1).unwrap().to_owned().to_ascii_lowercase();
  // 操作の中身によって取り出す値を変える
  let lend_type = match lend_type_str {
    "lend" => {
      // 貸出：「貸し出した品名」と「貸出先」
      let product = csv_record.get(2).unwrap().to_owned();
      let destination = csv_record.get(3).unwrap().to_owned();
      lib::LendType::Lend(product, destination)
    }
    "return" => {
      // 返却：「返された品名」と「返却先」
      let product = csv_record.get(2).unwrap().to_owned();
      let destination = csv_record.get(3).unwrap().to_owned();
      lib::LendType::Return(product, destination)
    }
    "edit" => {
      // 編集：「編集する操作対象に付けられた通し番号」と「編集後の品名」と「編集後の貸出先」
      let num = csv_record.get(4).unwrap().parse().unwrap();
      let new_product = csv_record.get(5).unwrap().to_owned();
      let new_destination = csv_record.get(6).unwrap().to_owned();
      lib::LendType::Edit(num, new_product, new_destination)
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

// 受け取ったCSVファイルのパスからCSVデータを取り出し、
// StringRecordのリストに直した後に貸出返却のデータ群に変換をかける
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

// 貸出返却のデータを受け取ってCSVファイルの中身を作成し、実際に出力するところまで行う
fn lend_data_lst_to_output(path: &str, csv_data_lst: Vec<lib::LendData>) {
  let empty_str = "";
  let mut wtr = Writer::from_path(path).unwrap();
  // ヘッダー部分
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
  // 貸出返却のデータは一つずつ書き込んでいく
  for lend_data in csv_data_lst.iter() {
    let time = lend_data.clone().time;
    let time_str = &time.to_rfc3339();
    let num_str = &lend_data.num.to_string();
    let lend_type = lend_data.clone().lend_type;
    match lend_type {
      // 貸出：「貸し出した品名」と「貸出先」
      lib::LendType::Lend(product_name, destination) => {
        wtr
          .write_record(&[
            time_str,
            "Lend",
            &product_name,
            &destination,
            empty_str,
            empty_str,
            empty_str,
            num_str,
          ])
          .unwrap();
      }
      // 返却：「返された品名」と「返却先」
      lib::LendType::Return(product_name, destination_string) => {
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
      lib::LendType::Edit(num, new_product_name, new_destination_string) => {
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
  // 書き終えたらこれで終了させる
  wtr.flush().unwrap()
}

// 貸出の重複や貸し出していないものの返却などを見つけるために、
// 「貸出す予定もしくは返却される予定の品名」とデータの中身の貸出品名が一致するかを探る関数
// データの中身がそもそもとして貸出以外の操作であった場合は「一致しない」を返すようにしている
fn check_lend_product_num(data: &lib::LendData, product_num: &str) -> bool {
  match data.clone().lend_type {
    lib::LendType::Lend(data_product_num, _) => data_product_num == *product_num,
    _ => false,
  }
}

// 貸出先と返却先が一致しているかチェックする
// 一致していたらtrue
// 一致していなかったり対象がなかったりしたらfalse
fn check_return_destination_num(
  data_lst: &[lib::LendData],
  product_num: &str,
  destination_num: &str,
) -> bool {
  match data_lst
    .iter()
    .find(|data| check_lend_product_num(data, &product_num))
  {
    Some(data) => match data.clone().lend_type {
      lib::LendType::Lend(_, n) => n == destination_num,
      _ => false,
    },
    None => false,
  }
}

#[test]
fn check_regex() {
  use regex::Regex;
  let re1 = Regex::new("\\d{4}").unwrap();
  assert_eq!(true, re1.is_match("0123"));
  assert_eq!(false, re1.is_match("1"));
  let re2 = Regex::new(".").unwrap();
  assert_eq!(true, re2.is_match("0123"));
  assert_eq!(true, re2.is_match("1"));
  let re = Regex::new("0\\d{3}").unwrap();
  assert_eq!(true, re.is_match("0123"));
  assert_eq!(false, re.is_match("1123"));
  assert_eq!(false, re.is_match("1"));
  let re = Regex::new("\\d{2}").unwrap();
  assert_eq!(true, re.is_match("0123"));
  assert_eq!(true, re.is_match("1123"));
  assert_eq!(false, re.is_match("1"));
  let re = Regex::new("0").unwrap();
  assert_eq!(true, re.is_match("0"));
  assert_eq!(true, re.is_match("60"));
  assert_eq!(false, re.is_match("1"));
}

// mainの関数
// ソフトウェアが実行された場合、この関数が実行され、他の関数を次々に呼び出して処理を行っていく
#[allow(unused_assignments)]
fn main() {
  // 引数の処理
  let matches = App::new("dlm")
    .version("0.2.0")
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

  // データを格納するCSVファイルのパスを受け取る
  let data_file_name_opt = matches.value_of("data_file_name");
  let data_file_name = match data_file_name_opt {
    Some(s) => s,
    None => {
      eprintln!("ファイル名を入力してください！");
      process::exit(1)
    }
  };

  // 対応させる名前が書かれたJSONファイルのパスを受け取る
  // 別に与えられていなくても大丈夫
  // JSONファイルが与えられていないときは空のリストを作成し、
  // JSONファイルが与えられている場合は中身を解析して
  // 品名の対応リストと団体の対応リストをそれぞれ作成してまとめる
  let config_file_name_opt = matches.value_of("config_file_name");
  let config_data: lib::ConfigData = match config_file_name_opt {
    None => lib::make_config_data(json!(null), json!(null), json!(null)),
    Some(config_file_name) => {
      let json_data = match path_to_json_data(config_file_name) {
        None => {
          eprintln!("JSONファイルの読み込み・解析に失敗しました");
          process::exit(1)
        }
        Some(v) => v,
      };
      let sizai_json_data = &json_data["sizai"];
      let sandan_json_data = &json_data["sandan"];
      let room_json_data = &json_data["room"];
      lib::make_config_data(
        sizai_json_data.clone(),
        sandan_json_data.clone(),
        room_json_data.clone(),
      )
    }
  };

  // 'history'コマンド用に、入力されたコマンドを記録するための空リストを作成しておく
  let mut arg_command_history_vec: Vec<String> = Vec::new();
  // このソフトウェアの目的や役割、リポジトリのURLなどの基本情報を出力する
  print_message::print_start();
  // 対話環境の開始
  // 操作を促すメッセージを表示し、コマンドが入力されたらそれに対応する処理をして反応を返し、
  // 処理が終わったらまた操作を促すメッセージを表示する、というループによって成り立っている
  // 'exit'コマンドが入力されたら正常にループから脱出して終了します
  // 実行時エラーでは以上終了します
  loop {
    // 操作を促すメッセージの表示
    print_message::print_restart();
    // コマンド文字列を標準入力から受け取ります
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).ok();
    // 受け取った文字列の前後から改行文字等を削除します
    s = s.trim().to_owned();
    // 綺麗にした文字列をコマンドを記録するリストに登録します
    arg_command_history_vec.push(s.clone());
    // 空白で区切ってリスト化し、コマンドと引数に対応するデータ構造を受け取ります
    let arg_str_vec: Vec<&str> = s.split_whitespace().collect();
    let arg = lib::parse_arg(arg_str_vec);
    // 引数のデータ構造に対応する処理と反応を行います
    match arg {
      // コメントや何も入力されなかったときは何もしないでループを回す
      lib::DlmArg::Null => (),
      // 'exit'はループから脱出して正常終了
      lib::DlmArg::Exit => break,
      // helpメッセージを表示
      lib::DlmArg::Help => print_message::print_help(),
      // 使えるコマンドではないというメッセージを表示して再度の入力を促す
      lib::DlmArg::NotFoundCommandName(name) => print_message::print_not_found_command_name(name),
      // 「引数の数や型が間違っている」ということを伝えて再度の入力を促す
      lib::DlmArg::MissingArgument(msg) => print_message::print_missing_argument(msg),
      // 記録していたコマンド文字列を表示する
      lib::DlmArg::History(n) => print_message::print_history(&arg_command_history_vec, n),
      // データを記録していたCSVファイルを読み込んでデータ群を抜き出し、
      // ヘッダーを出力した後に、データから作成した文字列を出力する
      lib::DlmArg::Show(re_opt) => {
        let lend_data = csv_file_name_to_lend_data(data_file_name.to_owned());
        let (lend_data_str, product_str_len_max) =
          lib::make_lend_data_str(lend_data, config_data.clone(), re_opt);
        println!(
          "操作番号   時刻               貸出品{}   貸出先（団体名）（場所）",
          " ".repeat(product_str_len_max - 6)
        );
        println!("{}", lend_data_str)
      }
      // データを記録していたCSVファイルを読み込んでデータ群を抜き出し、
      // 実際に貸出と返却の処理を仮想的に行いながら二重貸出等の間違いを探す
      // 間違いが検出されたらその中身を出力し、全てのデータについて検査し終わったら終了
      lib::DlmArg::Check => {
        println!("検査を開始します\n--- --- ---\n");
        // 操作の削除や編集を反映し終えて貸出と返却のみで構成されたデータ群を作成する
        let lend_data_lst =
          lib::organize_lend_data(&csv_file_name_to_lend_data(data_file_name.to_owned()));
        // 現在貸し出されている品名を記録するためのリスト
        let mut lend_stack: Vec<&lib::LendData> = Vec::new();
        // 全ての操作を順番に行う
        for lend_data in lend_data_lst.iter() {
          let lend_type = &lend_data.lend_type;
          match lend_type {
            lib::LendType::Lend(product_num, _) => {
              // これから貸し出そうとする品名が既に貸し出したものを記録したリストに無いかをチェックする
              // 存在したらメッセージを出力し、とりあえず再度登録しなおす
              match lend_stack
                .iter()
                .find(|data| check_lend_product_num(data, product_num))
              {
                None => (),
                Some(_) => eprintln!("- {}が2重に貸し出されています\n", product_num),
              }
              lend_stack.push(lend_data)
            }
            lib::LendType::Return(product_num, _) => {
              // 返却された品名が貸し出したものを記録したリストにきちんとあるかをチェックする
              // 無かった場合はメッセージを出力する
              match lend_stack
                .iter()
                .find(|data| check_lend_product_num(data, product_num))
              {
                None => eprintln!(
                  "- {}が貸し出されていないにもかかわらず返却されたことになっています\n",
                  product_num
                ),
                Some(_) => (),
              }
              // 「返却された品を含まないリストを作り直す」ことで「返却」という挙動を再現する
              let new_lend_stack = lend_stack
                .iter()
                .filter(|data| !(check_lend_product_num(data, product_num)))
                .cloned()
                .collect();
              lend_stack = new_lend_stack
            }
            // これ以外は無いはずなので考慮しない
            _ => {}
          }
        }
        println!("--- --- ---\n検査を終了しました\n");
      }
      lib::DlmArg::Lend(product_num_lst, destination_num) => {
        // CSVファイルへのパスから生のデータ群を取り出す
        let mut lend_data = csv_file_name_to_lend_data(data_file_name.to_owned());
        // データ群の中で最大の操作番号を探し出す。
        let lend_data_num_max = lend_data
          .iter()
          .max_by_key(|x| x.num)
          .map(|data_opt| data_opt.num)
          .unwrap_or(0);
        let mut lend_num = lend_data_num_max;
        // 貸出品のリストに対して
        //  - 現在時刻の取得
        //  - 検査
        // を行い、全部が検査を通った時に書き込む
        // 一つでも検査を通らなかったらエラーとして処理し、なにも書き込まない
        let mut check_is_ok = true;
        for product_num in product_num_lst.iter() {
          lend_num += 1;
          // 現在時刻をタイムゾーン分の9時間分ずらした上で取得
          let time_fixed_offset = Utc::now().with_timezone(&FixedOffset::east(9 * 3600));
          // 'check'コマンドへの処理でやったことと同じ検査を行う
          // 検査を通過したらリストに登録してファイル更新
          // 検査を通らなかったらメッセージを表示して終了
          if lib::make_now_lend_data_lst(&lend_data)
            .iter()
            .any(|data| check_lend_product_num(data, &product_num))
          {
            check_is_ok = false;
            eprintln!(
              "!  {}が既に貸し出されているのでこの操作を行うことはできません",
              product_num
            );
            break;
          } else {
            // 成功したものを一時リストに登録していく
            lend_data.push(lib::LendData {
              time: time_fixed_offset,
              lend_type: lib::LendType::Lend(product_num.clone(), destination_num.clone()),
              num: lend_num,
            });
          }
        }
        if check_is_ok {
          // 書き出し
          lend_data_lst_to_output(data_file_name, lend_data);
          // 成功メッセージの出力
          for product_num in product_num_lst.iter() {
            print_message::print_lend_success(&product_num, &destination_num, &lend_num);
          }
        } else {
          // 検査不合格が発声していた場合
          eprintln!("今回行われた操作は全て中止されました。再度正しい貸出を実行してください。\n")
        }
      }
      lib::DlmArg::Return(product_num_lst, destination_num) => {
        // Lendのときとほとんど同じ
        let mut lend_data = csv_file_name_to_lend_data(data_file_name.to_owned());
        let lend_data_num_max = lend_data
          .iter()
          .max_by_key(|x| x.num)
          .map(|data_opt| data_opt.num)
          .unwrap_or(0);
        let mut lend_num = lend_data_num_max;
        // 返却品のリストに対して
        //  - 現在時刻の取得
        //  - 検査
        // を行い、全部が検査を通った時に書き込む
        // 一つでも検査を通らなかったらエラーとして処理し、なにも書き込まない
        let mut check_is_ok = true;
        for product_num in product_num_lst.iter() {
          lend_num += 1;
          // 現在時刻をタイムゾーン分の9時間分ずらした上で取得
          let time_fixed_offset = Utc::now().with_timezone(&FixedOffset::east(9 * 3600));
          // 'check'コマンドへの処理でやったことと同じ検査を行う
          // 検査を通過したらリストに登録してファイル更新
          // 検査を通らなかったらメッセージを表示して終了
          if lib::make_now_lend_data_lst(&lend_data)
            .iter()
            .any(|data| check_lend_product_num(data, &product_num))
          {
            // 貸出先と返却先が一致していなかったら警告を出して終了
            if !check_return_destination_num(
              &lib::make_now_lend_data_lst(&lend_data),
              &product_num,
              &destination_num,
            ) {
              // 貸出先と返却先が一致していない
              check_is_ok = false;
              eprintln!(
                "!  {}の貸出先と返却先が一致していないため、この操作を行うことは出来ません",
                product_num
              );
              break;
            } else {
              // 一致していたので返却操作を登録
              lend_data.push(lib::LendData {
                time: time_fixed_offset,
                lend_type: lib::LendType::Return(product_num.clone(), destination_num.clone()),
                num: lend_num,
              });
            }
          } else {
            check_is_ok = false;
            eprintln!(
              "!  {}がまだ貸し出されてないのでこの操作を行うことは出来ません",
              product_num
            );
            break;
          }
        }
        if check_is_ok {
          // 書き出し
          lend_data_lst_to_output(data_file_name, lend_data);
          // 成功メッセージの出力

          for product_num in product_num_lst.iter() {
            print_message::print_return_success(&product_num, &destination_num, &lend_num);
          }
        } else {
          // 検査不合格が発声していた場合
          eprintln!("今回行われた操作は全て中止されました。再度正しい貸出を実行してください。\n")
        }
      }
      lib::DlmArg::Edit(num, new_product_num, new_destination_num) => {
        // Lendとほぼ同じだが、編集する対象の操作が未来のものであった場合は不正とみなしてメッセージを表示して終了
        // また、本当に意図した編集内容になっているかを確認するためのメッセージを表示する
        // 'n'または'N'が入力された場合のみ操作を中止するが、それ以外の任意の文字列だった場合は編集を行う
        let mut lend_data = csv_file_name_to_lend_data(data_file_name.to_owned());
        let lend_data_num_max = lend_data
          .iter()
          .max_by_key(|x| x.num)
          .map(|data_opt| data_opt.num)
          .unwrap_or(0);
        let lend_num = lend_data_num_max + 1;
        if num > lend_data_num_max {
          eprintln!("!  未来の操作を編集することは出来ません\n");
        } else {
          let time_fixed_offset = Utc::now().with_timezone(&FixedOffset::east(9 * 3600));
          let data = lib::get_lend_data(&lend_data, num).unwrap();
          match data.lend_type {
            // 対象がEditとRemoveの時は不正とみなして終了
            lib::LendType::Edit(_, _, _) | lib::LendType::Remove(_) => {
              eprintln!("!  'remove'または'edit'で行った操作を編集することは出来ません");
            }
            _ => {
              let data_str = lib::lend_data_to_message_with_config_data(&data, &config_data);
              println!(
                "{}\nという{}番の操作の品名を\"{}\"に、相手を\"{}\"に変更します\n本当に良いですか？[Y/n]\n    >",
                data_str,
                num,
                new_product_num,
                new_destination_num
              );
              let mut s = String::new();
              std::io::stdin().read_line(&mut s).ok();
              let s: &str = &s.trim().to_owned().to_ascii_lowercase();
              match s {
                "n" => println!("操作を中止しました"),
                _ => {
                  lend_data.push(lib::LendData {
                    time: time_fixed_offset,
                    lend_type: lib::LendType::Edit(
                      num,
                      new_product_num.clone(),
                      new_destination_num.clone(),
                    ),
                    num: lend_num,
                  });
                  lend_data_lst_to_output(data_file_name, lend_data);
                  print_message::print_edit_success(
                    &num,
                    &new_product_num,
                    &new_destination_num,
                    &lend_num,
                  );
                }
              }
            }
          }
        }
      }
      lib::DlmArg::Remove(num) => {
        // Editとほぼ同じ
        let mut lend_data = csv_file_name_to_lend_data(data_file_name.to_owned());
        let lend_data_num_max = lend_data
          .iter()
          .max_by_key(|x| x.num)
          .map(|data_opt| data_opt.num)
          .unwrap_or(0);
        let lend_num = lend_data_num_max + 1;
        if num > lend_data_num_max {
          eprintln!("!  未来の操作を削除することは出来ません\n");
        } else {
          let time_fixed_offset = Utc::now().with_timezone(&FixedOffset::east(9 * 3600));
          let data = lib::get_lend_data(&lend_data, num).unwrap();
          let data_str = lib::lend_data_to_message_with_config_data(&data, &config_data);
          println!(
            "{}\nという{}番の操作を無かったことにします\n本当に良いですか？[Y/n] >",
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
                num: lend_num,
              });
              lend_data_lst_to_output(data_file_name, lend_data);
              print_message::print_remove_success(&num, &lend_num)
            }
          }
        }
      }
      lib::DlmArg::AllPrint => {
        // CSVファイルへのパスから生成したデータ群を文字列化してそのまま出力
        let lend_data_lst = csv_file_name_to_lend_data(data_file_name.to_owned());
        for lend_data in lend_data_lst {
          println!(
            "{}",
            lib::lend_data_to_message_with_config_data(&lend_data, &config_data)
          )
        }
      }
    };
  }
}
