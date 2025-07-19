// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2025 rtldg <rtldg@protonmail.com>

#include "../../../srcwrtimer/extshared/src/extension.h"
#include "../../../srcwrtimer/extshared/src/coreident.hpp"


extern "C" {

void rust_setup_replay_thread();
void rust_KILL_replay_thread();

void rust_post_to_replay_thread(
	  IChangeableForward* forward // what to pass along to the callback
	, int value // what to pass along to the callback
	, const char* wrpath
	, const char* copypath
	, const char* header
	, size_t headersize
	, void* playerrecording
	, size_t totalframes
	, const char* sm_friendly_path
);

}


extern const sp_nativeinfo_t FloppyNatives[];


void MyExtension::OnHandleDestroy(HandleType_t type, void* object) {}
bool MyExtension::GetHandleApproxSize(HandleType_t type, void* object, unsigned int* size) { return false; }


bool Extension_OnLoad(char* error, size_t maxlength)
{
	rust_setup_replay_thread();

	sharesys->AddNatives(myself, FloppyNatives);
	return true;
}

void Extension_OnUnload()
{
	rust_KILL_replay_thread();
}

void Extension_OnAllLoaded() {}

static cell_t N_SRCWRFloppy_AsyncSaveReplay(IPluginContext* ctx, const cell_t* params)
{
	cell_t callback = params[1];
	int value = params[2];

	char *wrpath_friendly, *copypath_friendly, wrpath[PLATFORM_MAX_PATH]{}, copypath[PLATFORM_MAX_PATH]{};
	(void)ctx->LocalToString(params[3], &wrpath_friendly);
	(void)ctx->LocalToString(params[4], &copypath_friendly);
	if (wrpath_friendly[0]) smutils->BuildPath(Path_Game, wrpath, sizeof(wrpath), "%s", wrpath_friendly);
	if (copypath_friendly[0]) smutils->BuildPath(Path_Game, copypath, sizeof(copypath), "%s", copypath_friendly);

	char* header;
	(void)ctx->LocalToString(params[5], &header);
	int headersize = params[6];

	void* playerrecording;
	Handle_t playerrecording_handle = params[7];
	if (HandleError err = ReadHandleCoreIdent(playerrecording_handle, g_ArrayListType, &playerrecording); err != HandleError_None)
		return ctx->ThrowNativeError("Invalid ArrayList Handle %x (error %d)", playerrecording_handle, err);

	int totalframes = params[8];

	IChangeableForward* fw = forwards->CreateForwardEx(
		  NULL
		, ET_Ignore
		, 3
		, NULL
		, Param_Any // saved
		, Param_Any // value
		, Param_String // sPath
	);
	if (!fw || !fw->AddFunction(ctx, callback))
	{
		if (fw) forwards->ReleaseForward(fw);
		return ctx->ThrowNativeError("Failed to create callback forward");
	}

	rust_post_to_replay_thread(
		  fw
		, value
		, wrpath
		, copypath
		, header
		, headersize
		, playerrecording
		, totalframes
		, wrpath_friendly[0] ? wrpath_friendly : copypath_friendly
	);

	return 0; // native marked as void so return value doesn't matter...
}

extern const sp_nativeinfo_t FloppyNatives[] = {
	{"SRCWRFloppy_AsyncSaveReplay", N_SRCWRFloppy_AsyncSaveReplay},
	{NULL, NULL}
};
