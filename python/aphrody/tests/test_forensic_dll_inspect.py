# SPDX-License-Identifier: Apache-2.0
"""Tests for :mod:`aphrody.forensic.dll_inspect` (mocked LIEF)."""

from __future__ import annotations

import types as _t

from aphrody.forensic import dll_inspect


def _fake_lief_module(binary):
    return _t.SimpleNamespace(parse=lambda path: binary)


def _make_binary(*, is_dll=True, signed=True):
    entry = _t.SimpleNamespace(name="CreateFileW", is_ordinal=False, ordinal=0)
    imp = _t.SimpleNamespace(name="KERNEL32.dll", entries=[entry])
    export_sym = _t.SimpleNamespace(name="DllMain")
    export = _t.SimpleNamespace(entries=[export_sym])
    section = _t.SimpleNamespace(
        name=".text", virtual_size=4096, size=4096, entropy=6.1
    )
    chars = ["DLL"] if is_dll else ["EXECUTABLE_IMAGE"]
    header = _t.SimpleNamespace(machine="AMD64", characteristics_list=chars)
    opt = _t.SimpleNamespace(subsystem="WINDOWS_CUI")
    cert = _t.SimpleNamespace(subject="C=US, O=Google Inc")
    signer = _t.SimpleNamespace(cert=cert)
    sig = _t.SimpleNamespace(signers=[signer])
    return _t.SimpleNamespace(
        header=header,
        optional_header=opt,
        imports=[imp],
        get_export=lambda: export,
        sections=[section],
        has_signatures=lambda: signed,
        signatures=[sig] if signed else [],
    )


def test_inspect_pe_imports_exports():
    binary = _make_binary()
    rep = dll_inspect.inspect_pe("x.dll", lief_mod=_fake_lief_module(binary))
    assert rep.machine == "AMD64"
    assert rep.is_dll is True
    assert "KERNEL32.dll" in rep.import_dlls
    assert rep.imports["KERNEL32.dll"] == ["CreateFileW"]
    assert "DllMain" in rep.exports
    assert rep.sections[0]["entropy"] == 6.1


def test_inspect_pe_signature():
    rep = dll_inspect.inspect_pe(
        "x.dll", lief_mod=_fake_lief_module(_make_binary(signed=True))
    )
    assert rep.signed is True
    assert any("Google Inc" in s for s in rep.signers)


def test_inspect_pe_unsigned():
    rep = dll_inspect.inspect_pe(
        "x.dll", lief_mod=_fake_lief_module(_make_binary(signed=False))
    )
    assert rep.signed is False
    assert rep.signers == []


def test_inspect_pe_parse_failure():
    mod = _t.SimpleNamespace(parse=lambda p: None)
    rep = dll_inspect.inspect_pe("x.dll", lief_mod=mod)
    assert rep.error is not None


def test_inspect_pe_parse_raises():
    def _raise(p):
        raise RuntimeError("corrupt")

    mod = _t.SimpleNamespace(parse=_raise)
    rep = dll_inspect.inspect_pe("x.dll", lief_mod=mod)
    assert "corrupt" in rep.error


def test_is_pe_candidate():
    assert dll_inspect.is_pe_candidate("a.dll")
    assert dll_inspect.is_pe_candidate("a.exe")
    assert dll_inspect.is_pe_candidate("a.node")
    assert not dll_inspect.is_pe_candidate("a.py")


def test_inspect_entries_filters(tmp_path):
    e_dll = _t.SimpleNamespace(
        path="a.dll", ext="dll", is_dir=False, markers=[]
    )
    e_py = _t.SimpleNamespace(path="b.py", ext="py", is_dir=False, markers=[])
    reps = dll_inspect.inspect_entries(
        [e_dll, e_py], lief_mod=_fake_lief_module(_make_binary())
    )
    assert len(reps) == 1
    assert reps[0].path == "a.dll"


def test_inspect_entries_pe_marker_no_ext():
    e = _t.SimpleNamespace(
        path="server_bin", ext="", is_dir=False, markers=["pe"]
    )
    reps = dll_inspect.inspect_entries(
        [e], lief_mod=_fake_lief_module(_make_binary())
    )
    assert len(reps) == 1
