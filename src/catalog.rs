use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DecoratorInfo {
    pub name: &'static str,
    pub scope: &'static str,
    pub summary: &'static str,
    pub documentation: &'static str,
    pub insert_text: &'static str,
    pub is_function: bool,
    pub deprecated: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct DataTypeOption {
    pub name: &'static str,
    pub insert_text: &'static str,
    pub documentation: &'static str,
}

#[derive(Debug, Clone)]
pub struct DataTypeInfo {
    pub name: &'static str,
    pub summary: &'static str,
    pub documentation: &'static str,
    pub insert_text: Option<&'static str>,
    pub option_snippets: Option<&'static [DataTypeOption]>,
}

#[derive(Debug, Clone)]
pub struct ResolverInfo {
    pub name: &'static str,
    pub summary: &'static str,
    pub documentation: &'static str,
    pub insert_text: &'static str,
}

pub static ROOT_DECORATORS: &[DecoratorInfo] = &[
    DecoratorInfo {
        name: "envFlag",
        scope: "root",
        summary: "Deprecated environment flag decorator.",
        documentation: "Deprecated at v0.1. Use `@currentEnv=$APP_ENV` instead.",
        insert_text: "@envFlag=${1:APP_ENV}",
        is_function: false,
        deprecated: Some("Use @currentEnv instead."),
    },
    DecoratorInfo {
        name: "currentEnv",
        scope: "root",
        summary: "Sets the env var reference used to select environment-specific files.",
        documentation: "Usually used in `.env.schema`, for example `# @currentEnv=$APP_ENV`.",
        insert_text: "@currentEnv=$${1:APP_ENV}",
        is_function: false,
        deprecated: None,
    },
    DecoratorInfo {
        name: "defaultRequired",
        scope: "root",
        summary: "Controls whether items default to required, optional, or inferred.",
        documentation: "Valid values are `true`, `false`, or `infer`.",
        insert_text: "@defaultRequired=${1|infer,true,false|}",
        is_function: false,
        deprecated: None,
    },
    DecoratorInfo {
        name: "defaultSensitive",
        scope: "root",
        summary: "Controls whether items default to sensitive.",
        documentation: "Valid values are `true`, `false`, or `inferFromPrefix(PUBLIC_)`.",
        insert_text: "@defaultSensitive=${1|true,false,inferFromPrefix(PUBLIC_)|}",
        is_function: false,
        deprecated: None,
    },
    DecoratorInfo {
        name: "disable",
        scope: "root",
        summary: "Disables the current file, optionally conditionally.",
        documentation: "Can be set directly or with a boolean resolver like `forEnv(test)`.",
        insert_text: "@disable=${1|true,false|}",
        is_function: false,
        deprecated: None,
    },
    DecoratorInfo {
        name: "generateTypes",
        scope: "root",
        summary: "Generates types from the schema.",
        documentation: "Common usage: `# @generateTypes(lang=ts, path=./env.d.ts)`.",
        insert_text: "@generateTypes(lang=${1:ts}, path=${2:./env.d.ts})",
        is_function: true,
        deprecated: None,
    },
    DecoratorInfo {
        name: "import",
        scope: "root",
        summary: "Imports schema and values from another file or directory.",
        documentation: "Takes a path as the first positional arg. Optional named args include `enabled` and `allowMissing`.",
        insert_text: "@import(${1:./.env.shared})",
        is_function: true,
        deprecated: None,
    },
    DecoratorInfo {
        name: "plugin",
        scope: "root",
        summary: "Loads a plugin that can register decorators, types, and resolvers.",
        documentation: "Use the package name or identifier as the first argument.",
        insert_text: "@plugin(${1:@varlock/plugin-name})",
        is_function: true,
        deprecated: None,
    },
    DecoratorInfo {
        name: "redactLogs",
        scope: "root",
        summary: "Controls whether sensitive values are redacted in logs.",
        documentation: "Boolean decorator. Sensitive values are replaced with redacted output when enabled.",
        insert_text: "@redactLogs=${1|true,false|}",
        is_function: false,
        deprecated: None,
    },
    DecoratorInfo {
        name: "preventLeaks",
        scope: "root",
        summary: "Controls whether outgoing responses are scanned for secret leaks.",
        documentation: "Boolean decorator that enables leak-prevention checks.",
        insert_text: "@preventLeaks=${1|true,false|}",
        is_function: false,
        deprecated: None,
    },
    DecoratorInfo {
        name: "setValuesBulk",
        scope: "root",
        summary: "Injects many config values from a single data source.",
        documentation: "Common usage: `# @setValuesBulk(exec(\"vault kv get ...\"), format=json)`.",
        insert_text: "@setValuesBulk(${1:exec(\"command\")}, format=${2|json,env|})",
        is_function: true,
        deprecated: None,
    },
];

pub static ITEM_DECORATORS: &[DecoratorInfo] = &[
    DecoratorInfo {
        name: "required",
        scope: "item",
        summary: "Marks an item as required.",
        documentation: "Equivalent to `@required=true`. Can also be driven by boolean resolvers like `forEnv(...)`.",
        insert_text: "@required",
        is_function: false,
        deprecated: None,
    },
    DecoratorInfo {
        name: "optional",
        scope: "item",
        summary: "Marks an item as optional.",
        documentation: "Equivalent to `@required=false`.",
        insert_text: "@optional",
        is_function: false,
        deprecated: None,
    },
    DecoratorInfo {
        name: "sensitive",
        scope: "item",
        summary: "Marks an item as sensitive.",
        documentation: "Sensitive items are redacted and treated as secrets.",
        insert_text: "@sensitive",
        is_function: false,
        deprecated: None,
    },
    DecoratorInfo {
        name: "public",
        scope: "item",
        summary: "Marks an item as not sensitive.",
        documentation: "Equivalent to `@sensitive=false`.",
        insert_text: "@public",
        is_function: false,
        deprecated: None,
    },
    DecoratorInfo {
        name: "type",
        scope: "item",
        summary: "Sets the item data type.",
        documentation: "Accepts a data type name or configured type call like `string(minLength=5)`.",
        insert_text: "@type=${1:string}",
        is_function: false,
        deprecated: None,
    },
    DecoratorInfo {
        name: "example",
        scope: "item",
        summary: "Adds an example value.",
        documentation: "Use an example when the stored value should stay empty or secret.",
        insert_text: "@example=${1:\"example value\"}",
        is_function: false,
        deprecated: None,
    },
    DecoratorInfo {
        name: "docsUrl",
        scope: "item",
        summary: "Deprecated single docs URL decorator.",
        documentation: "Deprecated. Prefer `@docs(...)`, which supports multiple docs entries.",
        insert_text: "@docsUrl=${1:https://example.com/docs}",
        is_function: false,
        deprecated: Some("Use docs() instead."),
    },
    DecoratorInfo {
        name: "docs",
        scope: "item",
        summary: "Attaches documentation URLs to an item.",
        documentation: "Supports `@docs(url)` or `@docs(\"Label\", url)` and may be used multiple times.",
        insert_text: "@docs(${1:https://example.com/docs})",
        is_function: true,
        deprecated: None,
    },
    DecoratorInfo {
        name: "icon",
        scope: "item",
        summary: "Attaches an icon identifier to an item.",
        documentation: "Useful for generated docs and UI surfaces that show schema metadata.",
        insert_text: "@icon=${1:mdi:key}",
        is_function: false,
        deprecated: None,
    },
];

pub static DATA_TYPES: &[DataTypeInfo] = &[
    DataTypeInfo {
        name: "string",
        summary: "String value with optional length, casing, and pattern settings.",
        documentation: "Example: `@type=string(minLength=5, startsWith=pk-)`.",
        insert_text: Some("string"),
        option_snippets: Some(&[
            DataTypeOption {
                name: "minLength",
                insert_text: "minLength=${1:1}",
                documentation: "Minimum allowed string length.",
            },
            DataTypeOption {
                name: "maxLength",
                insert_text: "maxLength=${1:255}",
                documentation: "Maximum allowed string length.",
            },
            DataTypeOption {
                name: "isLength",
                insert_text: "isLength=${1:32}",
                documentation: "Exact required string length.",
            },
            DataTypeOption {
                name: "startsWith",
                insert_text: "startsWith=${1:prefix-}",
                documentation: "Required starting substring.",
            },
            DataTypeOption {
                name: "endsWith",
                insert_text: "endsWith=${1:-suffix}",
                documentation: "Required ending substring.",
            },
            DataTypeOption {
                name: "matches",
                insert_text: "matches=${1:\"^[A-Z0-9_]+$\"}",
                documentation: "Regex or string pattern to match.",
            },
            DataTypeOption {
                name: "toUpperCase",
                insert_text: "toUpperCase=${1|true,false|}",
                documentation: "Coerce the final value to uppercase.",
            },
            DataTypeOption {
                name: "toLowerCase",
                insert_text: "toLowerCase=${1|true,false|}",
                documentation: "Coerce the final value to lowercase.",
            },
            DataTypeOption {
                name: "allowEmpty",
                insert_text: "allowEmpty=${1|true,false|}",
                documentation: "Allow empty string values.",
            },
        ]),
    },
    DataTypeInfo {
        name: "number",
        summary: "Number with min/max, precision, and divisibility options.",
        documentation: "Example: `@type=number(min=0, max=100, precision=1)`.",
        insert_text: Some("number"),
        option_snippets: Some(&[
            DataTypeOption {
                name: "min",
                insert_text: "min=${1:0}",
                documentation: "Minimum allowed number.",
            },
            DataTypeOption {
                name: "max",
                insert_text: "max=${1:100}",
                documentation: "Maximum allowed number.",
            },
            DataTypeOption {
                name: "coerceToMinMaxRange",
                insert_text: "coerceToMinMaxRange=${1|true,false|}",
                documentation: "Clamp values into the allowed min/max range.",
            },
            DataTypeOption {
                name: "isDivisibleBy",
                insert_text: "isDivisibleBy=${1:1}",
                documentation: "Require divisibility by the given number.",
            },
            DataTypeOption {
                name: "isInt",
                insert_text: "isInt=${1|true,false|}",
                documentation: "Require integer values.",
            },
            DataTypeOption {
                name: "precision",
                insert_text: "precision=${1:2}",
                documentation: "Allowed decimal precision for non-integers.",
            },
        ]),
    },
    DataTypeInfo {
        name: "boolean",
        summary: "Boolean value.",
        documentation: "Accepts common truthy and falsy string values during coercion.",
        insert_text: Some("boolean"),
        option_snippets: None,
    },
    DataTypeInfo {
        name: "url",
        summary: "URL with optional HTTPS prepending and allowed-domain checks.",
        documentation: "Example: `@type=url(prependHttps=true)`.",
        insert_text: Some("url"),
        option_snippets: Some(&[
            DataTypeOption {
                name: "prependHttps",
                insert_text: "prependHttps=${1|true,false|}",
                documentation: "Automatically add `https://` when missing.",
            },
            DataTypeOption {
                name: "allowedDomains",
                insert_text: "allowedDomains=${1:\"example.com\"}",
                documentation: "Restrict the URL host to an allowed domain list.",
            },
        ]),
    },
    DataTypeInfo {
        name: "simple-object",
        summary: "JSON-like object value.",
        documentation: "Coerces plain objects or JSON strings into objects.",
        insert_text: Some("simple-object"),
        option_snippets: None,
    },
    DataTypeInfo {
        name: "enum",
        summary: "Restricted value list.",
        documentation: "Requires explicit options, for example `@type=enum(dev, preview, prod)`.",
        insert_text: Some("enum(${1:development}, ${2:preview}, ${3:production})"),
        option_snippets: None,
    },
    DataTypeInfo {
        name: "email",
        summary: "Email address.",
        documentation: "Example: `@type=email(normalize=true)`.",
        insert_text: Some("email"),
        option_snippets: Some(&[DataTypeOption {
            name: "normalize",
            insert_text: "normalize=${1|true,false|}",
            documentation: "Lowercase the email before validation.",
        }]),
    },
    DataTypeInfo {
        name: "ip",
        summary: "IPv4 or IPv6 address.",
        documentation: "Example: `@type=ip(version=4, normalize=true)`.",
        insert_text: Some("ip"),
        option_snippets: Some(&[
            DataTypeOption {
                name: "version",
                insert_text: "version=${1|4,6|}",
                documentation: "Restrict to IPv4 or IPv6.",
            },
            DataTypeOption {
                name: "normalize",
                insert_text: "normalize=${1|true,false|}",
                documentation: "Normalize the value before validation.",
            },
        ]),
    },
    DataTypeInfo {
        name: "port",
        summary: "Port number between 0 and 65535.",
        documentation: "Example: `@type=port(min=1024, max=9999)`.",
        insert_text: Some("port"),
        option_snippets: Some(&[
            DataTypeOption {
                name: "min",
                insert_text: "min=${1:1024}",
                documentation: "Minimum allowed port.",
            },
            DataTypeOption {
                name: "max",
                insert_text: "max=${1:9999}",
                documentation: "Maximum allowed port.",
            },
        ]),
    },
    DataTypeInfo {
        name: "semver",
        summary: "Semantic version string.",
        documentation: "Validates standard semver values like `1.2.3`.",
        insert_text: Some("semver"),
        option_snippets: None,
    },
    DataTypeInfo {
        name: "isoDate",
        summary: "ISO 8601 date string.",
        documentation: "Supports date strings with optional time and milliseconds.",
        insert_text: Some("isoDate"),
        option_snippets: None,
    },
    DataTypeInfo {
        name: "uuid",
        summary: "UUID string.",
        documentation: "Validates RFC4122 UUIDs.",
        insert_text: Some("uuid"),
        option_snippets: None,
    },
    DataTypeInfo {
        name: "md5",
        summary: "MD5 hash string.",
        documentation: "Validates 32-character hexadecimal MD5 values.",
        insert_text: Some("md5"),
        option_snippets: None,
    },
];

pub static RESOLVERS: &[ResolverInfo] = &[
    ResolverInfo {
        name: "concat",
        summary: "Concatenates multiple values into one string.",
        documentation: "Equivalent to string expansion with multiple segments.",
        insert_text: "concat(${1:\"prefix-\"}, ${2:$$OTHER})",
    },
    ResolverInfo {
        name: "fallback",
        summary: "Returns the first non-empty value.",
        documentation: "Useful for layered defaults and optional sources.",
        insert_text: "fallback(${1:$$PRIMARY}, ${2:$$SECONDARY}, ${3:\"default\"})",
    },
    ResolverInfo {
        name: "exec",
        summary: "Executes a command and uses stdout as the value.",
        documentation: "Trailing newlines are trimmed automatically.",
        insert_text: "exec(`${1:command}`)",
    },
    ResolverInfo {
        name: "ref",
        summary: "References another config item.",
        documentation: "Usually you can use `$ITEM` directly, but `ref()` is useful when composing functions.",
        insert_text: "ref(${1:\"OTHER_KEY\"})",
    },
    ResolverInfo {
        name: "regex",
        summary: "Creates a regular expression for use inside other functions.",
        documentation: "Intended for use in other resolvers like `remap()`.",
        insert_text: "regex(${1:\"^dev.*\"})",
    },
    ResolverInfo {
        name: "remap",
        summary: "Maps one value to another based on match rules.",
        documentation: "Use key/value remapping pairs after the source value.",
        insert_text: "remap(${1:$$SOURCE}, ${2:production}=${3:\"main\"})",
    },
    ResolverInfo {
        name: "forEnv",
        summary: "Resolves to true when the current environment matches.",
        documentation: "Requires `@currentEnv` to be set in the schema.",
        insert_text: "forEnv(${1:development})",
    },
    ResolverInfo {
        name: "eq",
        summary: "Checks whether two values are equal.",
        documentation: "Returns a boolean.",
        insert_text: "eq(${1:$$LEFT}, ${2:\"value\"})",
    },
    ResolverInfo {
        name: "if",
        summary: "Returns different values based on a boolean condition.",
        documentation: "Supports boolean-only usage or explicit true/false values.",
        insert_text: "if(${1:eq($$ENV, \"prod\")}, ${2:\"https://api.example.com\"}, ${3:\"https://staging-api.example.com\"})",
    },
    ResolverInfo {
        name: "not",
        summary: "Negates a value.",
        documentation: "Falsy values become `true`, truthy values become `false`.",
        insert_text: "not(${1:forEnv(production)})",
    },
    ResolverInfo {
        name: "isEmpty",
        summary: "Checks whether a value is undefined or empty.",
        documentation: "Useful for conditionals and optional env values.",
        insert_text: "isEmpty(${1:$$OPTIONAL_KEY})",
    },
    ResolverInfo {
        name: "inferFromPrefix",
        summary: "Special helper for `@defaultSensitive`.",
        documentation: "Used as `@defaultSensitive=inferFromPrefix(PUBLIC_)`.",
        insert_text: "inferFromPrefix(${1:PUBLIC_})",
    },
];

pub static DECORATORS_BY_NAME: Lazy<HashMap<&'static str, &'static DecoratorInfo>> =
    Lazy::new(|| {
        ROOT_DECORATORS
            .iter()
            .chain(ITEM_DECORATORS.iter())
            .map(|d| (d.name, d))
            .collect()
    });
