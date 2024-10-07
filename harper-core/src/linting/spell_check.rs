use hashbrown::HashMap;
use smallvec::ToSmallVec;

use super::lint::Suggestion;
use super::{Lint, LintKind, Linter};
use crate::{
    document::{self, Document},
    Span, Token, TokenKind,
};
use crate::{spell::suggest_correct_spelling, token};
use crate::{CharString, Dictionary, TokenStringExt};

pub struct SpellCheck<T>
where
    T: Dictionary,
{
    dictionary: T,
    word_cache: HashMap<CharString, Vec<CharString>>,
}

impl<T: Dictionary> SpellCheck<T> {
    pub fn new(dictionary: T) -> Self {
        Self {
            dictionary,
            word_cache: HashMap::new(),
        }
    }
}

impl<T: Dictionary> SpellCheck<T> {
    fn cached_suggest_correct_spelling(&mut self, word: &[char]) -> Vec<CharString> {
        let word = word.to_smallvec();

        self.word_cache
            .entry(word.clone())
            .or_insert_with(|| {
                // Back off until we find a match.
                let mut suggestions = Vec::new();
                let mut dist = 2;

                while suggestions.is_empty() && dist < 5 {
                    suggestions = suggest_correct_spelling(&word, 100, dist, &self.dictionary)
                        .into_iter()
                        .map(|v| v.to_smallvec())
                        .collect();

                    dist += 1;
                }

                suggestions
            })
            .clone()
    }
}

fn potentially_combine_unlintable_markdown_tokens(
    document: &Document,
    idx: usize,
) -> Option<(Token, Span)> {
    dbg!(idx);

    let missing_token = document.get_token(idx)?;
    let [punct_1, unlintable, punct_2] = document
        .get_tokens()
        .get(idx.saturating_sub(3)..idx)?
        .try_into()
        .ok()?;

    dbg!(punct_1, unlintable, punct_2, missing_token);

    // We require the unlintable token to be surrounded by punctuation.
    if !(punct_1.kind.is_open_square() && punct_2.kind.is_close_square()) {
        return None;
    }

    if !unlintable.kind.is_unlintable() {
        return None;
    }

    let mut new_token = missing_token;
    new_token.span = Span {
        start: punct_1.span.start,
        end: missing_token.span.end,
    };

    Some((new_token, unlintable.span))
}

impl<T: Dictionary> Linter for SpellCheck<T> {
    fn lint(&mut self, document: &Document) -> Vec<Lint> {
        let mut lints = Vec::new();

        for (idx, mut word) in document
            .tokens()
            .enumerate()
            .filter(|(_, t)| t.kind.is_word())
        {
            println!("checking: {:?}", word);

            let word_chars = document.get_span_content(word.span);
            if self.dictionary.contains_word(word_chars) {
                println!(
                    "dict contains, done: {:?}",
                    document.get_span_content(word.span)
                );
                continue;
            }

            // attempt to combine unlintable markdown tokens
            if let Some((new_token, mut unlintable_span)) =
                potentially_combine_unlintable_markdown_tokens(document, idx)
            {
                // todo: fix unlintable span creation to remove this hack
                unlintable_span.push_by(1);

                let extra_content = document.get_span_content(unlintable_span);
                let suffix_content = document.get_span_content(word.span);

                let check_word = [extra_content, suffix_content].concat();
                if self.dictionary.contains_word(&check_word) {
                    println!("dict contains check word: {:?}", check_word);
                    continue;
                } else {
                    word = new_token;
                }
            }

            let mut possibilities = self.cached_suggest_correct_spelling(word_chars);

            // only look at the first 3 suggestions
            possibilities.truncate(3);

            // If the misspelled word is capitalized, capitalize the results too.
            if let Some(mis_f) = word_chars.first() {
                if mis_f.is_uppercase() {
                    for sug_f in possibilities.iter_mut().filter_map(|w| w.first_mut()) {
                        *sug_f = sug_f.to_uppercase().next().unwrap();
                    }
                }
            }

            let suggestions = possibilities
                .into_iter()
                .map(|word| Suggestion::ReplaceWith(word.to_vec()));

            lints.push(Lint {
                span: word.span,
                lint_kind: LintKind::Spelling,
                suggestions: suggestions.collect(),
                message: format!(
                    "Did you mean to spell “{}” this way?",
                    document.get_span_content_str(word.span)
                ),
                priority: 63,
            })
        }

        lints
    }
}
