// Copyright 2021-2024 Anicet Ebou.
// Licensed under the MIT license (http://opensource.org/licenses/MIT)
// This file may not be copied, modified, or distributed except according
// to those terms.

use anyhow::{anyhow, Context};
use bio::io::fasta;
use bio::pattern_matching::myers::MyersBuilder;
use fern::colors::ColoredLevelConfig;
use log::{error, info, warn};
use phf::phf_map;

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};

pub fn setup_logging(quiet: bool) -> anyhow::Result<(), fern::InitError> {
    let colors = ColoredLevelConfig::default();
    let mut base_config = fern::Dispatch::new();

    base_config = match quiet {
        // if user required quietness let only output warning messages
        // or messages more severe than warnings
        true => base_config.level(log::LevelFilter::Warn),
        // if quietness is not specified which implies verbosity is allowed
        // output
        false => base_config.level(log::LevelFilter::Debug),
    };

    // Separate file config so we can include year, month and day in file logs
    let file_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(fern::log_file("hyperex.log")?);

    let stdout_config = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                chrono::Local::now().format("%H:%M:%S"),
                colors.color(record.level()),
                message
            ))
        })
        .chain(io::stdout());

    base_config
        .chain(file_config)
        .chain(stdout_config)
        .apply()?;

    Ok(())
}

// Primers data
static PRIMER_TO_REGION: phf::Map<&'static str, &'static str> = phf_map! {
    "AGAGTTTGATCMTGGCTCAG" => "v1",
    "CCTACGGGNGGCWGCAG" => "v3",
    "GTGCCAGCMGCCGCGGTAA" => "v4",
    "GTGYCAGCMGCCGCGGTAA" => "v4",
    "AACMGGATTAGATACCCKG" => "v5",
    "TAAAACTYAAAKGAATTGACGGGG" => "v6",
    "YAACGAGCGCAACCC" => "v7",
    "ACTGCTGCSYCCCGTAGGAGTCT" => "v2",
    "ATTACCGCGGCTGCTGG" => "v3",
    "GACTACHVGGGTATCTAATCC" => "v4",
    "CCGTCAATTYMTTTRAGT" => "v5",
    "GGACTACHVGGGTWTCTAAT" => "v4",
    "CCCCGYCAATTCMTTTRAGT" => "v5",
    "ACGTCATCCCCACCTTCC" => "v7",
    "TACGGYTACCTTGTTAYGACTT" => "v9"
};

static FORWARD_PRIMERS: phf::Map<&'static str, &'static str> = phf_map! {
    "27F" => "AGAGTTTGATCMTGGCTCAG",
    "341F" => "CCTACGGGNGGCWGCAG",
    "515F" => "GTGCCAGCMGCCGCGGTAA",
    "515F-Y" => "GTGYCAGCMGCCGCGGTAA",
    "799F" => "AACMGGATTAGATACCCKG",
    "928F" => "TAAAACTYAAAKGAATTGACGGGG",
    "1100F" => "YAACGAGCGCAACCC",
};

static REVERSE_PRIMERS: phf::Map<&'static str, &'static str> = phf_map! {
    "336R" => "ACTGCTGCSYCCCGTAGGAGTCT",
    "534R" => "ATTACCGCGGCTGCTGG",
    "805R" => "GACTACHVGGGTATCTAATCC",
    "926Rb" => "CCGTCAATTYMTTTRAGT",
    "806R" => "GGACTACHVGGGTWTCTAAT",
    "909-928R" => "CCCCGYCAATTCMTTTRAGT",
    "1193R" => "ACGTCATCCCCACCTTCC",
    "1492Rmod" => "TACGGYTACCTTGTTAYGACTT",
};

pub fn region_to_primer(region: &str) -> anyhow::Result<Vec<String>> {
    match region {
        "v1v2" => Ok(vec![
            FORWARD_PRIMERS["27F"].to_string(),
            REVERSE_PRIMERS["336R"].to_string(),
        ]),
        "v1v3" => Ok(vec![
            FORWARD_PRIMERS["27F"].to_string(),
            REVERSE_PRIMERS["534R"].to_string(),
        ]),
        "v1v9" => Ok(vec![
            FORWARD_PRIMERS["27F"].to_string(),
            REVERSE_PRIMERS["1492Rmod"].to_string(),
        ]),
        "v3v4" => Ok(vec![
            FORWARD_PRIMERS["341F"].to_string(),
            REVERSE_PRIMERS["805R"].to_string(),
        ]),
        "v3v5" => Ok(vec![
            FORWARD_PRIMERS["341F"].to_string(),
            REVERSE_PRIMERS["926Rb"].to_string(),
        ]),
        "v4" => Ok(vec![
            FORWARD_PRIMERS["515F"].to_string(),
            REVERSE_PRIMERS["806R"].to_string(),
        ]),
        "v4v5" => Ok(vec![
            FORWARD_PRIMERS["515F-Y"].to_string(),
            REVERSE_PRIMERS["909-928R"].to_string(),
        ]),
        "v5v7" => Ok(vec![
            FORWARD_PRIMERS["799F"].to_string(),
            REVERSE_PRIMERS["1193R"].to_string(),
        ]),
        "v6v9" => Ok(vec![
            FORWARD_PRIMERS["928F"].to_string(),
            REVERSE_PRIMERS["1492Rmod"].to_string(),
        ]),
        "v7v9" => Ok(vec![
            FORWARD_PRIMERS["1100F"].to_string(),
            REVERSE_PRIMERS["1492Rmod"].to_string(),
        ]),
        _ => Ok(vec!["".to_string()]),
    }
}

pub fn file_to_vec(filename: &str) -> anyhow::Result<Vec<Vec<String>>> {
    let mut vec: Vec<Vec<String>> = Vec::new();
    let content = fs::read_to_string(filename)?;
    for line in content.lines() {
        if line.contains(',') {
            vec.push(
                line.split(',')
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            );
        } else {
            return Err(anyhow!(
                "File containing primer sequences is not comma separated"
            ));
        }
    }
    Ok(vec)
}

pub fn combine_vec(first: Vec<&str>, second: Vec<&str>) -> Vec<Vec<String>> {
    first
        .iter()
        .zip(second)
        .map(|x| vec![x.0.to_string(), x.1.to_string()])
        .collect::<Vec<Vec<String>>>()
}

fn read_file(
    filename: &str,
) -> anyhow::Result<(Box<dyn io::Read>, niffler::compression::Format)> {
    let raw_in = Box::new(io::BufReader::new(File::open(filename)?));

    Ok(niffler::get_reader(raw_in)?)
}

fn primers_to_region(primers: Vec<String>) -> String {
    let mut first_part = "";
    let mut second_part = "";

    if PRIMER_TO_REGION.contains_key(&primers[0]) {
        first_part = PRIMER_TO_REGION[&primers[0]];
    }

    if PRIMER_TO_REGION.contains_key(&primers[1]) {
        second_part = PRIMER_TO_REGION[&primers[1]];
    }

    if first_part == "v4" && second_part == "v4" {
        first_part.to_string()
    } else {
        format!("{}{}", first_part, second_part)
    }
}

fn to_complement(primer: &str, alphabet: &str) -> String {
    let mut complement = String::new();

    if alphabet == "dna" {
        // S and W complements are themselves, they are therefore ignored here
        complement = primer
            .chars()
            .map(|x| match x {
                'A' => 'T',
                'T' => 'A',
                'C' => 'G',
                'G' => 'C',
                'R' => 'Y',
                'Y' => 'R',
                'K' => 'M',
                'M' => 'K',
                'B' => 'V',
                'V' => 'B',
                'D' => 'H',
                'H' => 'D',
                'N' => 'N',
                _ => x,
            })
            .collect();
    } else if alphabet == "rna" {
        complement = primer
            .chars()
            .map(|x| match x {
                'A' => 'U',
                'U' => 'A',
                'C' => 'G',
                'G' => 'C',
                'R' => 'Y',
                'Y' => 'R',
                'K' => 'M',
                'M' => 'K',
                'B' => 'V',
                'V' => 'B',
                'D' => 'H',
                'H' => 'D',
                'N' => 'N',
                _ => x,
            })
            .collect();
    }

    complement
}

fn to_reverse_complement(primer: &str, alphabet: &str) -> String {
    let complement = to_complement(primer, alphabet);
    let reverse_complement = complement.chars().rev().collect();

    reverse_complement
}

#[derive(Debug, PartialEq)]
pub enum Alphabet {
    Dna,
    Rna,
}

pub fn sequence_type(sequence: &str) -> Option<Alphabet> {
    let valid_dna_iupac = "ACGTRYSWKMBDHVN";
    let valid_rna_iupac = "ACGURYSWKMBDHVN";

    if sequence.chars().all(|x| valid_dna_iupac.contains(x)) {
        Some(Alphabet::Dna)
    } else if sequence.chars().all(|x| valid_rna_iupac.contains(x)) {
        Some(Alphabet::Rna)
    } else {
        None
    }
}

pub fn get_hypervar_regions(
    file: &str,
    primers: Vec<Vec<String>>,
    prefix: &str,
    mismatch: u8,
) -> anyhow::Result<()> {
    let (reader, mut _compression) =
        read_file(file).with_context(|| "Cannot read file")?;

    let mut records = fasta::Reader::new(reader).records();

    let mut fasta_writer = fasta::Writer::to_file(format!("{}.fa", prefix))?;
    let gff_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{}.gff", prefix))?;
    let mut gff_writer = io::BufWriter::new(gff_file);
    gff_writer.write_all(b"##gff-version 3\n")?;

    // Build Myers with IUPAC ambiguities in patterns
    let ambigs = [
        (b'M', &b"AC"[..]),
        (b'R', &b"AG"[..]),
        (b'W', &b"AT"[..]),
        (b'S', &b"CG"[..]),
        (b'Y', &b"CT"[..]),
        (b'K', &b"GT"[..]),
        (b'V', &b"ACGMRS"[..]),
        (b'H', &b"ACTMWY"[..]),
        (b'D', &b"AGTRWK"[..]),
        (b'B', &b"CGTSYK"[..]),
        (b'N', &b"ACGTMRWSYKVHDB"[..]),
    ];

    let mut builder = MyersBuilder::new();

    for &(base, equivalents) in &ambigs {
        builder.ambig(base, equivalents);
    }

    while let Some(Ok(record)) = records.next() {
        let seq = record.seq();
        let mut alphabet = "";
        match sequence_type(std::str::from_utf8(seq)?) {
            Some(alp) => {
                if alp == Alphabet::Dna {
                    info!("Sequence type is DNA");
                    alphabet = "dna";
                } else if alp == Alphabet::Rna {
                    info!("Sequence type is RNA");
                    alphabet = "rna";
                }
            }
            None => error!("Sequence type is not recognized as DNA or RNA"),
        }
        if seq.len() <= 1500 {
            warn!("Sequence length is less than 1500 bp. We may not be able to find some regions");
        }

        for primer_pair in primers.iter() {
            let region = primers_to_region(primer_pair.to_vec());

            let mut forward_myers = builder.build_64(primer_pair[0].as_bytes());
            let mut reverse_myers = builder.build_64(
                to_reverse_complement(&primer_pair[1], alphabet).as_bytes(),
            );

            let mut forward_matches =
                forward_myers.find_all_lazy(seq, mismatch);
            let mut reverse_matches =
                reverse_myers.find_all_lazy(seq, mismatch);

            // Get the best hit
            let forward_best_hit =
                forward_matches.by_ref().min_by_key(|&(_, dist)| dist);
            let reverse_best_hit =
                reverse_matches.by_ref().min_by_key(|&(_, dist)| dist);

            match forward_best_hit {
                Some((forward_best_hit_end, _)) => {
                    match reverse_best_hit {
                        Some((reverse_best_hit_end, _)) => {
                            // Get match start position of forward primer
                            let (forward_start, _) = forward_matches
                                .hit_at(forward_best_hit_end)
                                .unwrap();
                            // Get match start position of reverse primer
                            let (reverse_start, _) = reverse_matches
                                .hit_at(reverse_best_hit_end)
                                .unwrap();

                            if !region.is_empty() {
                                fasta_writer.write_record(
                                    &fasta::Record::with_attrs(
                                        record.id(),
                                        Some(
                                            format!(
                                            "region={} forward={} reverse={}",
                                            region,
                                            primer_pair[0],
                                            primer_pair[1]
                                        )
                                            .as_str(),
                                        ),
                                        &seq[forward_start
                                            ..reverse_start
                                                + primer_pair[1].len()],
                                    ),
                                )?;
                            } else {
                                fasta_writer.write_record(
                                    &fasta::Record::with_attrs(
                                        record.id(),
                                        Some(
                                            format!(
                                                "forward={} reverse={}",
                                                primer_pair[0], primer_pair[1]
                                            )
                                            .as_str(),
                                        ),
                                        &seq[forward_start
                                            ..reverse_start
                                                + primer_pair[1].len()],
                                    ),
                                )?;
                            }
                            // Write region to GFF3 file
                            gff_writer.write_all(format!("{}\thyperex\tregion\t{}\t{}\t.\t.\t.\tNote Hypervariable region {}\n", record.id(), forward_start, reverse_start + primer_pair[1].len(), region).as_bytes())?;
                        }
                        None => {
                            warn!("Region {} not found because primer {} was not found in the sequence", region, primer_pair[1])
                        }
                    }
                }
                None => match reverse_best_hit {
                    Some((_, _)) => {
                        warn!("Region {} not found because primer {} was not found in the sequence", region, primer_pair[0]);
                    }
                    None => {
                        warn!("Region {} not found because primers {}, {} was not found in the sequence", region, primer_pair[0], primer_pair[1])
                    }
                },
            }
        }
    }

    Ok(())
}

// Tests --------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_primers_to_region_ok() {
        assert_eq!(
            primers_to_region(vec![
                "CCTACGGGNGGCWGCAG".to_string(),
                "GTGCCAGCMGCCGCGGTAA".to_string()
            ]),
            "v3v4".to_string()
        );
    }

    #[test]
    fn test_primers_to_region_ok2() {
        assert_eq!(
            primers_to_region(vec![
                "GTGCCAGCMGCCGCGGTAA".to_string(),
                "GTGCCAGCMGCCGCGGTAA".to_string()
            ]),
            "v4".to_string()
        );
    }

    #[test]
    fn test_primers_to_region_empty() {
        assert_eq!(
            primers_to_region(vec!["ZZZZZ".to_string(), "AAAAAA".to_string()]),
            "".to_string()
        );
    }

    #[test]
    fn test_complement_dna() {
        assert_eq!(
            to_complement("ATCGATCGATCGATCGRYKBVDHX", "dna"),
            String::from("TAGCTAGCTAGCTAGCYRMVBHDX")
        );
    }

    #[test]
    fn test_complement_rna() {
        assert_eq!(
            to_complement("AUCGAUCGAUCGAUCGRYKBVDHMXN", "rna"),
            String::from("UAGCUAGCUAGCUAGCYRMVBHDKXN")
        );
    }

    #[test]
    fn test_reverse_complement() {
        assert_eq!(
            to_reverse_complement("GTGCCAGCMGCCGCGGTAAN", "dna"),
            "NTTACCGCGGCKGCTGGCAC"
        );
    }

    #[test]
    fn test_sequence_type_dna_ok() {
        assert_eq!(sequence_type("ATCGATCGATCG"), Some(Alphabet::Dna));
    }

    #[test]
    fn test_sequence_type_dna_iupac_ok() {
        assert_eq!(sequence_type("ATCGMTGCAATCG"), Some(Alphabet::Dna));
    }

    #[test]
    fn test_sequence_type_rna_ok() {
        assert_eq!(sequence_type("AGCUUUGCA"), Some(Alphabet::Rna));
    }

    #[test]
    fn test_sequence_type_rna_iupac_ok() {
        assert_eq!(sequence_type("GUUUUAACCCAAM"), Some(Alphabet::Rna));
    }

    #[test]
    fn test_sequence_type_err() {
        assert_eq!(sequence_type("ATCXXXRMGU"), None);
    }

    #[test]
    fn test_region_to_primer_ok() {
        assert_eq!(
            region_to_primer("v1v2").unwrap(),
            vec!["AGAGTTTGATCMTGGCTCAG", "ACTGCTGCSYCCCGTAGGAGTCT"]
        );
        assert_eq!(
            region_to_primer("v1v3").unwrap(),
            vec!["AGAGTTTGATCMTGGCTCAG", "ATTACCGCGGCTGCTGG"]
        );
        assert_eq!(
            region_to_primer("v1v9").unwrap(),
            vec!["AGAGTTTGATCMTGGCTCAG", "TACGGYTACCTTGTTAYGACTT"]
        );
        assert_eq!(
            region_to_primer("v3v4").unwrap(),
            vec!["CCTACGGGNGGCWGCAG", "GACTACHVGGGTATCTAATCC"]
        );
        assert_eq!(
            region_to_primer("v3v5").unwrap(),
            vec!["CCTACGGGNGGCWGCAG", "CCGTCAATTYMTTTRAGT"]
        );
        assert_eq!(
            region_to_primer("v4").unwrap(),
            vec!["GTGCCAGCMGCCGCGGTAA", "GGACTACHVGGGTWTCTAAT"]
        );
        assert_eq!(
            region_to_primer("v4v5").unwrap(),
            vec!["GTGYCAGCMGCCGCGGTAA", "CCCCGYCAATTCMTTTRAGT"]
        );
        assert_eq!(
            region_to_primer("v5v7").unwrap(),
            vec!["AACMGGATTAGATACCCKG", "ACGTCATCCCCACCTTCC"]
        );
        assert_eq!(
            region_to_primer("v6v9").unwrap(),
            vec!["TAAAACTYAAAKGAATTGACGGGG", "TACGGYTACCTTGTTAYGACTT"]
        );
        assert_eq!(
            region_to_primer("v7v9").unwrap(),
            vec!["YAACGAGCGCAACCC", "TACGGYTACCTTGTTAYGACTT"]
        );
        assert_eq!(region_to_primer("").unwrap(), vec![""]);
    }

    #[test]
    fn test_write_fa_ok2() {
        let mut tmpfile =
            NamedTempFile::new().expect("Cannot create temp file");
        writeln!(tmpfile, ">id_str desc\nATCGCCG")
            .expect("Cannot write to tmp file");

        let mut fa_records = fasta::Reader::from_file(tmpfile)
            .expect("Cannot read file.")
            .records();

        while let Some(Ok(rec)) = fa_records.next() {
            assert_eq!(rec.id(), "id_str");
            assert_eq!(rec.desc(), Some("desc"));
            assert_eq!(rec.seq(), b"ATCGCCG");
        }
    }

    #[test]
    fn test_combine_vec() {
        let first = vec!["ab", "cd", "ef"];
        let second = vec!["cd", "ef", "gh"];
        assert_eq!(
            combine_vec(first, second),
            vec![
                vec!["ab".to_string(), "cd".to_string()],
                vec!["cd".to_string(), "ef".to_string()],
                vec!["ef".to_string(), "gh".to_string()]
            ]
        );
    }

    #[test]
    fn test_combine_vec_not_ok() {
        let first = vec!["ab", "cd", "ef"];
        let second = vec!["ab"];
        assert_ne!(
            combine_vec(first, second),
            vec![
                vec!["ab".to_string(), "cd".to_string()],
                vec!["cd".to_string(), "ef".to_string()],
                vec!["ef".to_string(), "gh".to_string()]
            ]
        );
    }

    #[test]
    fn test_get_hypervar_regions() {
        assert!(get_hypervar_regions(
            "tests/test.fa.gz",
            vec![vec![
                "AGAGTTTGATCMTGGCTCAG".to_string(),
                "TACGGYTACCTTGTTAYGACTT".to_string()
            ]],
            "hyperex",
            0
        )
        .is_ok());
        fs::remove_file("hyperex.fa").expect("cannot delete file");
        fs::remove_file("hyperex.gff").expect("cannot delete file");
    }

    #[test]
    fn test_setup_logging() {
        assert!(setup_logging(false).is_ok());
    }

    #[test]
    fn test_read_file() {
        let myfile = "tests/test.fa.gz";
        assert!(read_file(myfile).is_ok());
    }

    #[test]
    fn test_file_to_vec() {
        assert_eq!(
            file_to_vec("tests/primers.txt").unwrap(),
            vec![
                vec![
                    "CCTACGGGNGGCWGCAG".to_string(),
                    "ATTACCGCGGCTGCTGG".to_string()
                ],
                vec![
                    "GTGCCAGCMGCCGCGGTAA".to_string(),
                    "GACTACHVGGGTATCTAATCC".to_string()
                ]
            ]
        );
    }

    #[test]
    fn test_file_to_vec_no_ok() {
        assert!(file_to_vec("test.fa").is_err());
    }
}
