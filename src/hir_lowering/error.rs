/*
 * ******************************************************************************************
 * Copyright (c) 2019 Pascal Kuthe. This file is part of the VARF project.
 * It is subject to the license terms in the LICENSE file found in the top-level directory
 *  of this distribution and at  https://gitlab.com/DSPOM/VARF/blob/master/LICENSE.
 *  No part of VARF, including this file, may be copied, modified, propagated, or
 *  distributed except according to the terms contained in the LICENSE file.
 * *****************************************************************************************
 */

use annotate_snippets::display_list::{DisplayList, FormatOptions};
use annotate_snippets::snippet::{Annotation, AnnotationType, Slice, Snippet, SourceAnnotation};
use log::*;

use crate::ir::{ParameterId, VariableId};
use crate::parser::error::translate_to_inner_snippet_range;
use crate::symbol::Symbol;
use crate::{Hir, SourceMap};
use beef::lean::Cow;

pub type Error<'hir> = crate::error::Error<Type<'hir>>;
pub(crate) type Warning<'hir> = crate::error::Error<WarningType<'hir>>;
pub type Result<'hir, T = ()> = std::result::Result<T, Error<'hir>>;

#[derive(Clone, Debug)]
pub enum Type<'hir> {
    ExpectedReal,
    CannotCompareStringToNumber,
    CondtionTypeMissmatch,
    ExpectedInteger,
    ExpectedNumber,
    ExpectedIntegerParameter(ParameterId<'hir>),
    ExpectedIntegerVariable(VariableId<'hir>),
    ExpectedNumericParameter(ParameterId<'hir>),
    ParameterDefinedAfterConstantReference(ParameterId<'hir>),
    InvalidParameterBound,
    OnlyNumericExpressionsCanBeDerived,
    ParameterExcludeNotPartOfRange,
    ImplicitSolverDeltaIsNotAValidString,
}
impl<'tag> Error<'tag> {
    pub fn print(&self, source_map: &SourceMap, hir: &Hir<'tag>, translate_lines: bool) {
        let (line, line_number, substitution_name, range) =
            source_map.resolve_span_within_line(self.source, translate_lines);
        let (origin, mut footer) = if let Some(substitution_name) = substitution_name {
            (Cow::owned(substitution_name),vec![Annotation{
                id: None,
                label: Some("If macros/files are included inside this macro/file the error output might be hard to understand/display incorrect line numbers (See fully expanded source)"),
                annotation_type: AnnotationType::Note
            }])
        } else {
            (Cow::const_str(source_map.main_file_name), Vec::new())
        };
        let opt = FormatOptions {
            color: true,
            anonymized_line_numbers: false,
        };

        match self.error_type {
            Type::ExpectedReal => {
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some("Expected real valued expression"),
                        annotation_type: AnnotationType::Error,
                    }),
                    footer,
                    slices: vec![Slice {
                        source: line,
                        line_start: line_number as usize,
                        origin: Some(&*origin),
                        annotations: vec![SourceAnnotation {
                            range,
                            label: "Expected real",
                            annotation_type: AnnotationType::Error,
                        }],
                        fold: false,
                    }],
                    opt,
                };
                let display_list = DisplayList::from(snippet);
                error!("{}", display_list);
            }

            Type::ExpectedInteger => {
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some("Expected integer valued expression"),
                        annotation_type: AnnotationType::Error,
                    }),
                    footer,
                    slices: vec![Slice {
                        source: line,
                        line_start: line_number as usize,
                        origin: Some(&*origin),
                        annotations: vec![SourceAnnotation {
                            range,
                            label: "Expected integer",
                            annotation_type: AnnotationType::Error,
                        }],
                        fold: false,
                    }],
                    opt,
                };
                let display_list = DisplayList::from(snippet);
                error!("{}", display_list);
            }

            Type::ExpectedNumber => {
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some("Expected integer an numerical valued expression"),
                        annotation_type: AnnotationType::Error,
                    }),
                    footer,
                    slices: vec![Slice {
                        source: line,
                        line_start: line_number as usize,
                        origin: Some(&*origin),
                        annotations: vec![SourceAnnotation {
                            range,
                            label: "Expected numerical value",
                            annotation_type: AnnotationType::Error,
                        }],
                        fold: false,
                    }],
                    opt,
                };
                let display_list = DisplayList::from(snippet);
                error!("{}", display_list);
            }

            Type::ExpectedIntegerVariable(variable) => {
                let (
                    parameter_line,
                    parameter_line_number,
                    parameter_substitution_name,
                    parameter_range,
                ) = source_map.resolve_span_within_line(hir[variable].source, translate_lines);
                let parameter_origin = if let Some(substitution_name) = parameter_substitution_name
                {
                    Cow::owned(substitution_name)
                } else {
                    Cow::const_str(source_map.main_file_name)
                };
                let parameter_range = translate_to_inner_snippet_range(
                    parameter_range.start,
                    parameter_range.end,
                    parameter_line,
                );
                let parameter_line = parameter_line;
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let label = format!("{} is declared here", hir[variable].contents.name);
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some("Expected an integer valued variable"),
                        annotation_type: AnnotationType::Error,
                    }),
                    footer,
                    slices: vec![
                        Slice {
                            source: line,
                            line_start: line_number as usize,
                            origin: Some(&*origin),
                            annotations: vec![SourceAnnotation {
                                range,
                                label: "Expected integer",
                                annotation_type: AnnotationType::Error,
                            }],
                            fold: false,
                        },
                        Slice {
                            source: parameter_line,
                            line_start: parameter_line_number as usize,
                            origin: Some(&*parameter_origin),
                            annotations: vec![SourceAnnotation {
                                range: parameter_range,
                                label: &label,
                                annotation_type: AnnotationType::Info,
                            }],
                            fold: false,
                        },
                    ],
                    opt,
                };
                let display_list = DisplayList::from(snippet);
                error!("{}", display_list);
            }

            Type::ExpectedIntegerParameter(parameter) => {
                let (
                    parameter_line,
                    parameter_line_number,
                    parameter_substitution_name,
                    parameter_range,
                ) = source_map.resolve_span_within_line(hir[parameter].source, translate_lines);
                let parameter_origin = if let Some(substitution_name) = parameter_substitution_name
                {
                    Cow::owned(substitution_name)
                } else {
                    Cow::const_str(source_map.main_file_name)
                };
                let parameter_range = translate_to_inner_snippet_range(
                    parameter_range.start,
                    parameter_range.end,
                    parameter_line,
                );
                let parameter_line = parameter_line;
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let label = format!("{} is declared here", hir[parameter].contents.name);
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some("Expected an integer valued parameter"),
                        annotation_type: AnnotationType::Error,
                    }),
                    footer,
                    slices: vec![
                        Slice {
                            source: line,
                            line_start: line_number as usize,
                            origin: Some(&*origin),
                            annotations: vec![SourceAnnotation {
                                range,
                                label: "Expected integer",
                                annotation_type: AnnotationType::Error,
                            }],
                            fold: false,
                        },
                        Slice {
                            source: parameter_line,
                            line_start: parameter_line_number as usize,
                            origin: Some(&*parameter_origin),
                            annotations: vec![SourceAnnotation {
                                range: parameter_range,
                                label: &label,
                                annotation_type: AnnotationType::Info,
                            }],
                            fold: false,
                        },
                    ],
                    opt,
                };
                let display_list = DisplayList::from(snippet);
                error!("{}", display_list);
            }

            Type::ExpectedNumericParameter(parameter) => {
                let (
                    parameter_line,
                    parameter_line_number,
                    parameter_substitution_name,
                    parameter_range,
                ) = source_map.resolve_span_within_line(hir[parameter].source, translate_lines);
                let parameter_origin = if let Some(substitution_name) = parameter_substitution_name
                {
                    Cow::owned(substitution_name)
                } else {
                    Cow::const_str(source_map.main_file_name)
                };
                let parameter_range = translate_to_inner_snippet_range(
                    parameter_range.start,
                    parameter_range.end,
                    parameter_line,
                );
                let parameter_line = parameter_line;
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let label = format!(
                    "Expected numeric parameter but {} is a String",
                    hir[parameter].contents.name
                );
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some(&label),
                        annotation_type: AnnotationType::Error,
                    }),
                    footer,
                    slices: vec![
                        Slice {
                            source: line,
                            line_start: line_number as usize,
                            origin: Some(&*origin),
                            annotations: vec![SourceAnnotation {
                                range,
                                label: "Expected numeric value",
                                annotation_type: AnnotationType::Error,
                            }],
                            fold: false,
                        },
                        Slice {
                            source: parameter_line,
                            line_start: parameter_line_number as usize,
                            origin: Some(&*parameter_origin),
                            annotations: vec![SourceAnnotation {
                                range: parameter_range,
                                label: "Parameter declared here",
                                annotation_type: AnnotationType::Info,
                            }],
                            fold: false,
                        },
                    ],
                    opt,
                };
                let display_list = DisplayList::from(snippet);
                error!("{}", display_list);
            }

            Type::ParameterDefinedAfterConstantReference(parameter) => {
                let (
                    parameter_line,
                    parameter_line_number,
                    parameter_substitution_name,
                    parameter_range,
                ) = source_map.resolve_span_within_line(hir[parameter].source, translate_lines);
                let parameter_origin = if let Some(substitution_name) = parameter_substitution_name
                {
                    Cow::owned(substitution_name)
                } else {
                    Cow::const_str(source_map.main_file_name)
                };
                let parameter_range = translate_to_inner_snippet_range(
                    parameter_range.start,
                    parameter_range.end,
                    parameter_line,
                );
                let parameter_line = parameter_line;
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let label = format!(
                    "Parameter {} was referenced before it was defined in a constant context",
                    hir[parameter].contents.name
                );
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some(&label),
                        annotation_type: AnnotationType::Error,
                    }),
                    footer,
                    slices: vec![
                        Slice {
                            source: line,
                            line_start: line_number as usize,
                            origin: Some(&*origin),
                            annotations: vec![SourceAnnotation {
                                range,
                                label: "Constant reference here",
                                annotation_type: AnnotationType::Error,
                            }],
                            fold: false,
                        },
                        Slice {
                            source: parameter_line,
                            line_start: parameter_line_number as usize,
                            origin: Some(&*parameter_origin),
                            annotations: vec![SourceAnnotation {
                                range: parameter_range,
                                label: "Parameter declared here",
                                annotation_type: AnnotationType::Info,
                            }],
                            fold: false,
                        },
                    ],
                    opt,
                };
                let display_list = DisplayList::from(snippet);
                error!("{}", display_list);
            }
            Type::InvalidParameterBound => {
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some(
                            "Invalid parameter range; Lower bound must be smaller than upper bound",
                        ),
                        annotation_type: AnnotationType::Error,
                    }),
                    footer,
                    slices: vec![Slice {
                        source: line,
                        line_start: line_number as usize,
                        origin: Some(&*origin),
                        annotations: vec![SourceAnnotation {
                            range,
                            label: "Lower bound must but smaller than upper bound",
                            annotation_type: AnnotationType::Error,
                        }],
                        fold: false,
                    }],
                    opt,
                };
                let display_list = DisplayList::from(snippet);
                error!("{}", display_list);
            }
            Type::ParameterExcludeNotPartOfRange => {
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some(
                            "Invalid parameter bound. Can not exclude a value that is not part of the bound to begin with"
                                ,
                        ),
                        annotation_type: AnnotationType::Error,
                    }),
                    footer,
                    slices: vec![Slice {
                        source: line,
                        line_start: line_number as usize,
                        origin:Some(&*origin),
                        annotations: vec![SourceAnnotation {
                            range,
                            label: "This calue can't be excluded",
                            annotation_type: AnnotationType::Error,
                        }],
                        fold: false,
                    }],
                    opt
                };
                let display_list = DisplayList::from(snippet);
                error!("{}", display_list);
            }
            Type::CannotCompareStringToNumber => {
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some("Strings cannot be compared to numbers"),
                        annotation_type: AnnotationType::Error,
                    }),
                    footer,
                    slices: vec![Slice {
                        source: line,
                        line_start: line_number as usize,
                        origin: Some(&*origin),
                        annotations: vec![SourceAnnotation {
                            range,
                            label: "String is compared to number",
                            annotation_type: AnnotationType::Error,
                        }],
                        fold: false,
                    }],
                    opt,
                };
                let display_list = DisplayList::from(snippet);
                error!("{}", display_list);
            }
            Type::CondtionTypeMissmatch => {
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some("Conditions must ever evaluate to a String or a Number"),
                        annotation_type: AnnotationType::Error,
                    }),
                    footer,
                    slices: vec![Slice {
                        source: line,
                        line_start: line_number as usize,
                        origin: Some(&*origin),
                        annotations: vec![SourceAnnotation {
                            range,
                            label: "Evaluates to number or string",
                            annotation_type: AnnotationType::Error,
                        }],
                        fold: false,
                    }],
                    opt,
                };
                let display_list = DisplayList::from(snippet);
                error!("{}", display_list);
            }
            Type::OnlyNumericExpressionsCanBeDerived => {
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some("Only numerical expressions can be taken derivatives of"),
                        annotation_type: AnnotationType::Error,
                    }),
                    footer,
                    slices: vec![Slice {
                        source: line,
                        line_start: line_number as usize,
                        origin: Some(&*origin),
                        annotations: vec![SourceAnnotation {
                            range,
                            label: "Tried to derive string",
                            annotation_type: AnnotationType::Error,
                        }],
                        fold: false,
                    }],
                    opt,
                };
                let display_list = DisplayList::from(snippet);
                error!("{}", display_list);
            }

            Type::ImplicitSolverDeltaIsNotAValidString => {
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some("Illegal ImplicitFunctionSolver attribute value. Expected the name of the delta as string constant"),
                        annotation_type: AnnotationType::Error,
                    }),
                    footer,
                    slices: vec![Slice {
                        source: line,
                        line_start: line_number as usize,
                        origin: Some(&*origin),
                        annotations: vec![SourceAnnotation {
                            range,
                            label: "Expected a string constant",
                            annotation_type: AnnotationType::Error,
                        }],
                        fold: false,
                    }],
                    opt,
                };
                let display_list = DisplayList::from(snippet);
                error!("{}", display_list);
            }
        };
    }
}
#[derive(Clone, Debug)]
pub enum WarningType<'hir> {
    ImplicitDerivative(VariableId<'hir>),
    SpecifiedDeltaIsNotDerived(Symbol),
}
impl<'tag> Warning<'tag> {
    pub fn print(&self, source_map: &SourceMap, hir: &Hir<'tag>, translate_lines: bool) {
        let (line, line_number, substitution_name, range) =
            source_map.resolve_span_within_line(self.source, translate_lines);
        let (origin, mut footer) = if let Some(substitution_name) = substitution_name {
            (Cow::owned(substitution_name),vec![Annotation{
                id: None,
                label: Some("If macros/files are included inside this macro/file the error output might be hard to understand/display incorrect line numbers (See fully expanded source)"),
                annotation_type: AnnotationType::Note
            }])
        } else {
            (Cow::const_str(source_map.main_file_name), Vec::new())
        };
        let opt = FormatOptions {
            color: true,
            anonymized_line_numbers: false,
        };

        match self.error_type {
            WarningType::ImplicitDerivative(var) => {
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let main_label = format!("Implicit derivatives can not be calculated using automatic differentiation! The derivative of {} is calulated inside a branch that depends on its value",hir[var].contents.name);
                let inline_label =
                    format!("Branch depends on the value of {} ", hir[var].contents.name);
                footer.push(Annotation{
                    id: None,
                    label: Some("This is an problem under active development please see https://gitlab.com/DSPOM/verilogae/-/issues/21 for the current status"),
                    annotation_type: AnnotationType::Note
                });
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some(main_label.as_str()),
                        annotation_type: AnnotationType::Warning,
                    }),
                    footer,
                    slices: vec![Slice {
                        source: line,
                        line_start: line_number as usize,
                        origin: Some(&*origin),
                        annotations: vec![SourceAnnotation {
                            range,
                            label: inline_label.as_str(),
                            annotation_type: AnnotationType::Warning,
                        }],
                        fold: false,
                    }],
                    opt,
                };
                let display_list = DisplayList::from(snippet);
                warn!("{}", display_list);
            }

            WarningType::SpecifiedDeltaIsNotDerived(name) => {
                let range = translate_to_inner_snippet_range(range.start, range.end, &line);
                let main_label = format!("The delta \"{}\" that was specified for the ImplicitFunctionSolver attribute is not the name of a Variable that is derived inside the loop",name);
                let inline_label = format!("{} is not derived inside the loop", name);
                footer.push(Annotation{
                    id: None,
                    label: Some("You can ignore this warning if you just added the attribute so that derivatives can potentially be taken in the future"),
                    annotation_type: AnnotationType::Help
                });
                let snippet = Snippet {
                    title: Some(Annotation {
                        id: None,
                        label: Some(main_label.as_str()),
                        annotation_type: AnnotationType::Warning,
                    }),
                    footer,
                    slices: vec![Slice {
                        source: line,
                        line_start: line_number as usize,
                        origin: Some(&*origin),
                        annotations: vec![SourceAnnotation {
                            range,
                            label: inline_label.as_str(),
                            annotation_type: AnnotationType::Warning,
                        }],
                        fold: false,
                    }],
                    opt,
                };
                let display_list = DisplayList::from(snippet);
                warn!("{}", display_list);
            }
        };
    }
}
